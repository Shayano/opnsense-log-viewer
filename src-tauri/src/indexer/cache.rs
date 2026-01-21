//! Persistent Index Cache for Instant Reload
//!
//! Story 6.4: Implements a cache system that allows previously indexed files
//! to reload instantly (<500ms) from disk cache instead of re-indexing.
//!
//! ## Cache Flow
//! 1. Calculate efficient file hash (first 1MB + last 1MB + size)
//! 2. Check if cache file exists at `{app_data}/index_cache/{hash}.rkyv`
//! 3. On hit: Load cached TieredIndex via rkyv zero-copy deserialization
//! 4. On miss: Run progressive indexation, then save to cache

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rkyv::{AlignedVec, Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use bytecheck::CheckBytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::indexer::hybrid::IndexError;
use crate::indexer::tiered::{HotIndexRkyv, TieredConfig, TieredIndex};

/// Magic bytes for cache file identification
const CACHE_MAGIC: &[u8; 9] = b"OPNSCACHE";

/// Current cache file format version
const CACHE_VERSION: u16 = 1;

/// Chunk size for file hash calculation (1 MB)
const HASH_CHUNK_SIZE: u64 = 1024 * 1024;

/// Cache metadata stored in cache files
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[archive_attr(derive(CheckBytes))]
#[serde(rename_all = "camelCase")]
pub struct CacheMetadata {
    /// Original source file path
    pub source_path: String,
    /// Unix timestamp when cache was created
    pub created_at: i64,
    /// Number of log entries in the cached index
    pub entry_count: u64,
    /// SHA256 hash of the source file (for validation)
    pub source_hash: [u8; 32],
}

/// Event payload for cache hit/miss events (Tauri IPC)
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexCacheEvent {
    /// Path to the source log file
    pub file_path: String,
    /// Whether cache lookup was successful
    pub cache_hit: bool,
    /// Cache age in seconds (only for cache hits)
    pub cache_age_seconds: Option<u64>,
    /// Reason for cache miss (only for cache misses)
    pub reason: Option<String>,
}

/// Persistent index cache manager
///
/// Manages cached TieredIndex files in the application data directory.
/// Cache files are named by the source file's content hash.
pub struct IndexCache {
    /// Directory where cache files are stored
    cache_directory: PathBuf,
}

impl IndexCache {
    /// Create a new IndexCache with the specified cache directory
    ///
    /// # Arguments
    /// * `app_data_dir` - Application data directory (cache will be in `{app_data_dir}/index_cache/`)
    pub fn new(app_data_dir: PathBuf) -> Self {
        let cache_directory = app_data_dir.join("index_cache");
        Self { cache_directory }
    }

    /// Ensure the cache directory exists
    fn ensure_cache_dir(&self) -> Result<(), IndexError> {
        if !self.cache_directory.exists() {
            fs::create_dir_all(&self.cache_directory)?;
            log::info!(
                "[CACHE] Created cache directory: {}",
                self.cache_directory.display()
            );
        }
        Ok(())
    }

    /// Get the cache file path for a given file hash
    pub fn get_cache_path(&self, file_hash: &[u8; 32]) -> PathBuf {
        let hash_hex = hex::encode(file_hash);
        self.cache_directory.join(format!("{}.rkyv", hash_hex))
    }

    /// Get cached index for a file if it exists and is valid
    ///
    /// # Arguments
    /// * `file_path` - Path to the source log file
    ///
    /// # Returns
    /// * `Ok(Some(TieredIndex))` - Cache hit, returns the cached index
    /// * `Ok(None)` - Cache miss (file not cached or invalid)
    /// * `Err(IndexError)` - Error during cache lookup
    pub fn get_cached_index(&self, file_path: &Path) -> Result<Option<(TieredIndex, CacheMetadata)>, IndexError> {
        // Early return if cache directory doesn't exist (no cached indexes yet)
        if !self.cache_directory.exists() {
            log::debug!(
                "[CACHE] Cache directory doesn't exist: {}",
                self.cache_directory.display()
            );
            return Ok(None);
        }

        // Verify source file exists before calculating hash (graceful handling for moved/deleted files)
        if !file_path.exists() {
            log::debug!(
                "[CACHE] Source file not found, cache lookup skipped: {}",
                file_path.display()
            );
            return Ok(None);
        }

        // Calculate file hash
        let file_hash = calculate_file_hash(file_path)?;
        let cache_path = self.get_cache_path(&file_hash);

        if !cache_path.exists() {
            log::debug!(
                "[CACHE] Cache miss: no cache file for {}",
                file_path.display()
            );
            return Ok(None);
        }

        // Load cache file
        let cache_data = fs::read(&cache_path)?;

        // Validate minimum size (magic + version + some data)
        if cache_data.len() < CACHE_MAGIC.len() + 2 {
            log::warn!(
                "[CACHE] Cache file too small, invalidating: {}",
                cache_path.display()
            );
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }

        // Validate magic bytes
        if &cache_data[..CACHE_MAGIC.len()] != CACHE_MAGIC {
            log::warn!(
                "[CACHE] Invalid cache magic, invalidating: {}",
                cache_path.display()
            );
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }

        // Read version
        let version_offset = CACHE_MAGIC.len();
        let version = u16::from_le_bytes([
            cache_data[version_offset],
            cache_data[version_offset + 1],
        ]);

        if version != CACHE_VERSION {
            log::warn!(
                "[CACHE] Unsupported cache version {} (expected {}), invalidating",
                version,
                CACHE_VERSION
            );
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }

        // Read metadata length (u32)
        let metadata_len_offset = version_offset + 2;
        if cache_data.len() < metadata_len_offset + 4 {
            log::warn!("[CACHE] Cache file truncated, invalidating");
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }
        let metadata_len = u32::from_le_bytes([
            cache_data[metadata_len_offset],
            cache_data[metadata_len_offset + 1],
            cache_data[metadata_len_offset + 2],
            cache_data[metadata_len_offset + 3],
        ]) as usize;

        // Validate metadata region
        let metadata_offset = metadata_len_offset + 4;
        if cache_data.len() < metadata_offset + metadata_len {
            log::warn!("[CACHE] Cache file truncated (metadata), invalidating");
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }

        // Deserialize metadata with rkyv - copy to aligned buffer first
        let metadata_bytes = &cache_data[metadata_offset..metadata_offset + metadata_len];
        let mut aligned_metadata = AlignedVec::with_capacity(metadata_len);
        aligned_metadata.extend_from_slice(metadata_bytes);

        let metadata: CacheMetadata = match rkyv::check_archived_root::<CacheMetadata>(&aligned_metadata) {
            Ok(archived) => archived
                .deserialize(&mut rkyv::Infallible)
                .map_err(|e| IndexError::SerializationError(format!("Metadata deserialization failed: {:?}", e)))?,
            Err(e) => {
                log::warn!("[CACHE] Invalid metadata in cache file: {:?}", e);
                let _ = fs::remove_file(&cache_path);
                return Ok(None);
            }
        };

        // Validate entry count
        if metadata.entry_count == 0 {
            log::warn!("[CACHE] Cache has 0 entries, invalidating");
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }

        // Read index data length (u64)
        let index_len_offset = metadata_offset + metadata_len;
        if cache_data.len() < index_len_offset + 8 {
            log::warn!("[CACHE] Cache file truncated (index length), invalidating");
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }
        let index_len = u64::from_le_bytes([
            cache_data[index_len_offset],
            cache_data[index_len_offset + 1],
            cache_data[index_len_offset + 2],
            cache_data[index_len_offset + 3],
            cache_data[index_len_offset + 4],
            cache_data[index_len_offset + 5],
            cache_data[index_len_offset + 6],
            cache_data[index_len_offset + 7],
        ]) as usize;

        // Validate index data region
        let index_offset = index_len_offset + 8;
        if cache_data.len() < index_offset + index_len {
            log::warn!("[CACHE] Cache file truncated (index data), invalidating");
            let _ = fs::remove_file(&cache_path);
            return Ok(None);
        }

        // Deserialize HotIndexRkyv - copy to aligned buffer for rkyv
        let index_bytes = &cache_data[index_offset..index_offset + index_len];
        let mut aligned_index = AlignedVec::with_capacity(index_len);
        aligned_index.extend_from_slice(index_bytes);

        let hot_index = match rkyv::check_archived_root::<HotIndexRkyv>(&aligned_index) {
            Ok(archived) => {
                // Convert directly from archived form to HotIndex
                archived.to_hot_index()
            }
            Err(e) => {
                log::warn!("[CACHE] Invalid index data in cache file: {:?}", e);
                let _ = fs::remove_file(&cache_path);
                return Ok(None);
            }
        };

        // Create TieredIndex from HotIndex
        let tiered_index = TieredIndex {
            config: TieredConfig::default(),
            hot: hot_index,
            warm: Vec::new(), // Warm tiers are not cached (referenced by path)
            total_entries: metadata.entry_count,
        };

        log::info!(
            "[CACHE] Cache hit: loaded {} entries from {}",
            metadata.entry_count,
            cache_path.display()
        );

        Ok(Some((tiered_index, metadata)))
    }

    /// Save a TieredIndex to the cache
    ///
    /// Uses atomic write pattern: write to `.rkyv.tmp` then rename to `.rkyv`
    ///
    /// # Arguments
    /// * `file_path` - Path to the source log file
    /// * `index` - The TieredIndex to cache
    pub fn save_index(&self, file_path: &Path, index: &TieredIndex) -> Result<(), IndexError> {
        self.ensure_cache_dir()?;

        // Calculate file hash
        let file_hash = calculate_file_hash(file_path)?;
        let cache_path = self.get_cache_path(&file_hash);
        let tmp_path = cache_path.with_extension("rkyv.tmp");

        // Create metadata
        let created_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let metadata = CacheMetadata {
            source_path: file_path.to_string_lossy().to_string(),
            created_at,
            entry_count: index.total_entries,
            source_hash: file_hash,
        };

        // Convert HotIndex to HotIndexRkyv for serialization
        let hot_index_rkyv = HotIndexRkyv::from_hot_index(&index.hot);

        // Serialize metadata
        let metadata_bytes = rkyv::to_bytes::<_, 256>(&metadata)
            .map_err(|e| IndexError::SerializationError(format!("Metadata serialization failed: {:?}", e)))?;

        // Serialize index
        let index_bytes = rkyv::to_bytes::<_, 256>(&hot_index_rkyv)
            .map_err(|e| IndexError::SerializationError(format!("Index serialization failed: {:?}", e)))?;

        // Write to temp file
        let file = File::create(&tmp_path)?;
        let mut writer = BufWriter::with_capacity(256 * 1024, file);

        // Write magic
        writer.write_all(CACHE_MAGIC)?;

        // Write version (u16 LE)
        writer.write_all(&CACHE_VERSION.to_le_bytes())?;

        // Write metadata length (u32 LE) and data
        writer.write_all(&(metadata_bytes.len() as u32).to_le_bytes())?;
        writer.write_all(&metadata_bytes)?;

        // Write index length (u64 LE) and data
        writer.write_all(&(index_bytes.len() as u64).to_le_bytes())?;
        writer.write_all(&index_bytes)?;

        writer.flush()?;
        drop(writer);

        // Atomic rename
        fs::rename(&tmp_path, &cache_path)?;

        log::info!(
            "[CACHE] Saved {} entries to cache: {} ({} bytes)",
            index.total_entries,
            cache_path.display(),
            CACHE_MAGIC.len() + 2 + 4 + metadata_bytes.len() + 8 + index_bytes.len()
        );

        Ok(())
    }

    /// Invalidate (delete) cached index for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the source log file
    pub fn invalidate(&self, file_path: &Path) -> Result<(), IndexError> {
        let file_hash = calculate_file_hash(file_path)?;
        let cache_path = self.get_cache_path(&file_hash);

        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
            log::info!(
                "[CACHE] Invalidated cache for: {}",
                file_path.display()
            );
        }

        Ok(())
    }

    /// Get cache age in seconds for a file, if cached
    pub fn get_cache_age(&self, file_path: &Path) -> Result<Option<u64>, IndexError> {
        let file_hash = calculate_file_hash(file_path)?;
        let cache_path = self.get_cache_path(&file_hash);

        if !cache_path.exists() {
            return Ok(None);
        }

        let cache_metadata = fs::metadata(&cache_path)?;
        let modified = cache_metadata.modified()?;
        let age = SystemTime::now()
            .duration_since(modified)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(Some(age))
    }

    /// Clear all cached indexes
    pub fn clear_all(&self) -> Result<u64, IndexError> {
        if !self.cache_directory.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in fs::read_dir(&self.cache_directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "rkyv").unwrap_or(false) {
                fs::remove_file(&path)?;
                count += 1;
            }
        }

        log::info!("[CACHE] Cleared {} cache files", count);
        Ok(count)
    }
}

/// Calculate efficient file hash for cache lookup
///
/// Uses SHA256 on (first 1MB + last 1MB + file size) for fast hash
/// calculation that still uniquely identifies file content.
///
/// # Performance
/// - Completes in <1 second for any file size
/// - Reads at most 2MB from disk regardless of file size
///
/// # Arguments
/// * `path` - Path to the file to hash
///
/// # Returns
/// * `Ok([u8; 32])` - SHA256 hash
/// * `Err(IndexError)` - IO error
pub fn calculate_file_hash(path: &Path) -> Result<[u8; 32], IndexError> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();

    if file_size <= 2 * HASH_CHUNK_SIZE {
        // Small file: hash entire content
        let mut content = Vec::with_capacity(file_size as usize);
        reader.read_to_end(&mut content)?;
        hasher.update(&content);
    } else {
        // Large file: hash first 1MB + last 1MB

        // Read first 1MB
        let mut first_chunk = vec![0u8; HASH_CHUNK_SIZE as usize];
        reader.read_exact(&mut first_chunk)?;
        hasher.update(&first_chunk);

        // Seek to last 1MB
        reader.seek(SeekFrom::End(-(HASH_CHUNK_SIZE as i64)))?;

        // Read last 1MB
        let mut last_chunk = vec![0u8; HASH_CHUNK_SIZE as usize];
        reader.read_exact(&mut last_chunk)?;
        hasher.update(&last_chunk);
    }

    // Include file size in hash
    hasher.update(&file_size.to_le_bytes());

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::indexer::inverted::InvertedIndex;
    use crate::indexer::bitmap::BitmapIndex;
    use crate::indexer::offset_table::OffsetTable;
    use crate::indexer::tiered::HotIndex;

    fn create_test_tiered_index(entry_count: u64) -> TieredIndex {
        let mut inverted = InvertedIndex::new();
        let mut bitmap = BitmapIndex::new();
        let mut offsets = OffsetTable::new();

        for i in 0..entry_count {
            let ip = format!("192.168.1.{}", i % 255);
            inverted.add_entry(i, Some(&ip), Some("10.0.0.1"), Some(443), Some(80));
            bitmap.add_entry(
                i,
                Some(if i % 2 == 0 { "block" } else { "pass" }),
                Some("TCP"),
                Some("vtnet0"),
            );
            offsets.add_offset(i, i * 100);
        }

        let hot = HotIndex::from_indexes(inverted, bitmap, offsets, 0, entry_count);

        TieredIndex {
            config: TieredConfig::default(),
            hot,
            warm: Vec::new(),
            total_entries: entry_count,
        }
    }

    #[test]
    fn test_calculate_file_hash_small_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("small.log");
        fs::write(&path, "small content for testing").unwrap();

        let hash = calculate_file_hash(&path).unwrap();
        assert_eq!(hash.len(), 32);

        // Hash should be deterministic
        let hash2 = calculate_file_hash(&path).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_calculate_file_hash_large_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("large.log");

        // Create 3MB file (larger than 2MB threshold)
        let content = vec![0u8; 3 * 1024 * 1024];
        fs::write(&path, &content).unwrap();

        let hash = calculate_file_hash(&path).unwrap();
        assert_eq!(hash.len(), 32);

        // Hash should be deterministic
        let hash2 = calculate_file_hash(&path).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_calculate_file_hash_different_content_different_hash() {
        let temp = TempDir::new().unwrap();
        let path1 = temp.path().join("file1.log");
        let path2 = temp.path().join("file2.log");

        fs::write(&path1, "content A").unwrap();
        fs::write(&path2, "content B").unwrap();

        let hash1 = calculate_file_hash(&path1).unwrap();
        let hash2 = calculate_file_hash(&path2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_file_hash_same_content_same_hash() {
        let temp = TempDir::new().unwrap();
        let path1 = temp.path().join("file1.log");
        let path2 = temp.path().join("file2.log");

        fs::write(&path1, "identical content").unwrap();
        fs::write(&path2, "identical content").unwrap();

        let hash1 = calculate_file_hash(&path1).unwrap();
        let hash2 = calculate_file_hash(&path2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_calculate_file_hash_exactly_2mb() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("exact2mb.log");

        // Create exactly 2MB file (boundary case)
        let content = vec![b'X'; 2 * 1024 * 1024];
        fs::write(&path, &content).unwrap();

        let hash = calculate_file_hash(&path).unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_index_cache_new() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        let hash = [0u8; 32];
        let expected_path = temp.path().join("index_cache").join(format!("{}.rkyv", hex::encode(hash)));
        assert_eq!(cache.get_cache_path(&hash), expected_path);
    }

    #[test]
    fn test_cache_miss_scenario() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create a log file
        let log_path = temp.path().join("test.log");
        fs::write(&log_path, "test content").unwrap();

        // Should be cache miss
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_hit_scenario() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create a log file
        let log_path = temp.path().join("test.log");
        fs::write(&log_path, "test content for caching").unwrap();

        // Create test index
        let index = create_test_tiered_index(1000);

        // Save to cache
        cache.save_index(&log_path, &index).unwrap();

        // Load from cache
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_some());

        let (loaded_index, metadata) = result.unwrap();
        assert_eq!(loaded_index.total_entries, 1000);
        assert_eq!(metadata.entry_count, 1000);
    }

    #[test]
    fn test_cache_invalidation() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create a log file
        let log_path = temp.path().join("test.log");
        fs::write(&log_path, "test content").unwrap();

        // Save to cache
        let index = create_test_tiered_index(500);
        cache.save_index(&log_path, &index).unwrap();

        // Verify cache exists
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_some());

        // Invalidate
        cache.invalidate(&log_path).unwrap();

        // Should be cache miss now
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_file_change_detection() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create a log file
        let log_path = temp.path().join("test.log");
        fs::write(&log_path, "original content").unwrap();

        // Save to cache
        let index = create_test_tiered_index(100);
        cache.save_index(&log_path, &index).unwrap();

        // Verify cache hit
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_some());

        // Modify file content (changes hash)
        fs::write(&log_path, "modified content - different length").unwrap();

        // Should be cache miss (different hash)
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_corrupted_cache_handling() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create a log file
        let log_path = temp.path().join("test.log");
        fs::write(&log_path, "test content").unwrap();

        // Calculate hash to get cache path
        let file_hash = calculate_file_hash(&log_path).unwrap();
        let cache_path = cache.get_cache_path(&file_hash);

        // Create cache directory and write corrupted cache file
        fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        fs::write(&cache_path, "corrupted data").unwrap();

        // Should handle gracefully (cache miss, auto-invalidate)
        let result = cache.get_cached_index(&log_path).unwrap();
        assert!(result.is_none());

        // Corrupted file should be deleted
        assert!(!cache_path.exists());
    }

    #[test]
    fn test_cache_age() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create a log file
        let log_path = temp.path().join("test.log");
        fs::write(&log_path, "test content").unwrap();

        // No cache yet
        let age = cache.get_cache_age(&log_path).unwrap();
        assert!(age.is_none());

        // Save to cache
        let index = create_test_tiered_index(100);
        cache.save_index(&log_path, &index).unwrap();

        // Cache age should be very small (just created)
        let age = cache.get_cache_age(&log_path).unwrap();
        assert!(age.is_some());
        assert!(age.unwrap() < 5); // Less than 5 seconds
    }

    #[test]
    fn test_clear_all_caches() {
        let temp = TempDir::new().unwrap();
        let cache = IndexCache::new(temp.path().to_path_buf());

        // Create multiple log files and cache them
        for i in 0..3 {
            let log_path = temp.path().join(format!("test{}.log", i));
            fs::write(&log_path, format!("content {}", i)).unwrap();
            let index = create_test_tiered_index(100);
            cache.save_index(&log_path, &index).unwrap();
        }

        // Clear all
        let count = cache.clear_all().unwrap();
        assert_eq!(count, 3);

        // All should be cache miss now
        for i in 0..3 {
            let log_path = temp.path().join(format!("test{}.log", i));
            let result = cache.get_cached_index(&log_path).unwrap();
            assert!(result.is_none());
        }
    }
}
