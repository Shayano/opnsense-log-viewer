//! Story 6.6: Cache Invalidation Integration Tests
//!
//! This file contains integration tests that validate the cache invalidation
//! behavior implemented in Story 6.4. These tests verify:
//!
//! - AC3: Cache invalidation when source file is modified
//! - AC3: Cache hit when source file is unchanged
//! - AC3: Proper handling of corrupted cache files
//! - AC3: Cache file hash calculation for different file sizes

use opnsense_log_viewer_lib::indexer::{IndexCache, TieredIndex, TieredConfig, calculate_file_hash};
use opnsense_log_viewer_lib::indexer::inverted::InvertedIndex;
use opnsense_log_viewer_lib::indexer::bitmap::BitmapIndex;
use opnsense_log_viewer_lib::indexer::offset_table::OffsetTable;
use std::fs::{self, File};
use std::io::Write;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Create a test TieredIndex with specified entry count
fn create_test_tiered_index(entry_count: u64) -> TieredIndex {
    let mut inverted = InvertedIndex::new();
    let mut bitmap = BitmapIndex::new();
    let mut offsets = OffsetTable::new();

    for i in 0..entry_count {
        let ip = format!("192.168.{}.{}", (i / 256) % 256, i % 256);
        inverted.add_entry(i, Some(&ip), Some("10.0.0.1"), Some(443), Some(80));
        bitmap.add_entry(
            i,
            Some(if i % 2 == 0 { "block" } else { "pass" }),
            Some("TCP"),
            Some("vtnet0"),
        );
        offsets.add_offset(i, i * 100);
    }

    let config = TieredConfig::with_hot_entries(entry_count + 1000);
    TieredIndex::from_hybrid(inverted, bitmap, offsets, entry_count, config)
        .expect("Failed to create TieredIndex")
}

/// Story 6.6 (AC3): Test that cache invalidates when source file content changes
#[test]
fn test_cache_invalidation_on_content_change() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    // Create initial log file
    let log_path = temp_dir.path().join("test.log");
    fs::write(&log_path, "original log content line 1\noriginal line 2\n")
        .expect("Failed to write log file");

    // Save index to cache
    let index = create_test_tiered_index(1000);
    cache.save_index(&log_path, &index).expect("Failed to save to cache");

    // Verify cache hit with original content
    let result = cache.get_cached_index(&log_path).expect("Cache lookup failed");
    assert!(result.is_some(), "Should get cache hit with original content");

    let (loaded_index, metadata) = result.unwrap();
    assert_eq!(loaded_index.total_entries, 1000);
    eprintln!(
        "[TEST] Cache hit: {} entries, created_at={}",
        metadata.entry_count, metadata.created_at
    );

    // Modify the source file content
    fs::write(&log_path, "modified log content - completely different!\nline 2 also changed\n")
        .expect("Failed to modify log file");

    // Cache lookup should now return None (different hash)
    let result_after_modify = cache.get_cached_index(&log_path)
        .expect("Cache lookup failed");
    assert!(
        result_after_modify.is_none(),
        "Should get cache miss after content modification"
    );

    eprintln!("[TEST] Cache miss after content change (as expected)");
}

/// Story 6.6 (AC3): Test cache hit when source file is unchanged
#[test]
fn test_cache_hit_unchanged_source() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    // Create log file
    let log_path = temp_dir.path().join("stable.log");
    let content = "stable content that won't change\nline 2\nline 3\n";
    fs::write(&log_path, content).expect("Failed to write log file");

    // Save index
    let index = create_test_tiered_index(500);
    cache.save_index(&log_path, &index).expect("Failed to save");

    // Multiple cache lookups should all hit
    for i in 0..5 {
        let result = cache.get_cached_index(&log_path).expect("Cache lookup failed");
        assert!(result.is_some(), "Cache lookup {} should hit", i);
    }

    eprintln!("[TEST] 5 consecutive cache hits on unchanged file");
}

/// Story 6.6 (AC3): Test cache handles file size changes
#[test]
fn test_cache_invalidation_on_size_change() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    let log_path = temp_dir.path().join("growing.log");

    // Create small file
    fs::write(&log_path, "small content").expect("Failed to write");

    let index = create_test_tiered_index(100);
    cache.save_index(&log_path, &index).expect("Failed to save");

    // Verify hit
    let result = cache.get_cached_index(&log_path).expect("Lookup failed");
    assert!(result.is_some());

    // Append to file (changes both size and content)
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&log_path)
        .expect("Failed to open for append");
    writeln!(file, "\nappended content that makes file larger").expect("Failed to append");
    drop(file);

    // Cache should miss due to changed hash
    let result_after_append = cache.get_cached_index(&log_path).expect("Lookup failed");
    assert!(
        result_after_append.is_none(),
        "Should miss after appending to file"
    );

    eprintln!("[TEST] Cache correctly invalidated on file size change");
}

/// Story 6.6 (AC3): Test cache handles corrupted cache files gracefully
#[test]
fn test_cache_corrupted_file_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    let log_path = temp_dir.path().join("corrupt_test.log");
    fs::write(&log_path, "test content for corruption test").expect("Failed to write");

    // Save valid index first
    let index = create_test_tiered_index(200);
    cache.save_index(&log_path, &index).expect("Failed to save");

    // Verify hit
    assert!(cache.get_cached_index(&log_path).unwrap().is_some());

    // Get cache file path and corrupt it
    let file_hash = calculate_file_hash(&log_path).expect("Hash failed");
    let cache_path = cache.get_cache_path(&file_hash);

    // Write garbage to cache file
    fs::write(&cache_path, "completely invalid garbage data that is not rkyv format")
        .expect("Failed to corrupt cache");

    // Cache lookup should handle gracefully (return None, delete corrupted file)
    let result = cache.get_cached_index(&log_path).expect("Lookup failed");
    assert!(
        result.is_none(),
        "Should return None for corrupted cache file"
    );

    // Corrupted file should be auto-deleted
    assert!(
        !cache_path.exists(),
        "Corrupted cache file should be deleted"
    );

    eprintln!("[TEST] Corrupted cache file handled gracefully and deleted");
}

/// Story 6.6 (AC3): Test file hash is deterministic
#[test]
fn test_file_hash_deterministic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("hash_test.log");

    // Create file with fixed content
    fs::write(&file_path, "fixed content for hash testing\nline 2\nline 3")
        .expect("Failed to write");

    // Calculate hash multiple times
    let mut hashes = Vec::new();
    for _ in 0..10 {
        let hash = calculate_file_hash(&file_path).expect("Hash calculation failed");
        hashes.push(hash);
    }

    // All hashes should be identical
    let first_hash = hashes[0];
    for (i, hash) in hashes.iter().enumerate() {
        assert_eq!(
            hash, &first_hash,
            "Hash calculation {} produced different result",
            i
        );
    }

    eprintln!("[TEST] File hash is deterministic (10/10 identical)");
}

/// Story 6.6 (AC3): Test file hash differs for different content
#[test]
fn test_file_hash_content_sensitive() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let file1 = temp_dir.path().join("file1.log");
    let file2 = temp_dir.path().join("file2.log");

    fs::write(&file1, "content A").expect("Failed to write file1");
    fs::write(&file2, "content B").expect("Failed to write file2");

    let hash1 = calculate_file_hash(&file1).expect("Hash failed");
    let hash2 = calculate_file_hash(&file2).expect("Hash failed");

    assert_ne!(hash1, hash2, "Different content should produce different hashes");

    eprintln!("[TEST] Different content produces different hashes");
}

/// Story 6.6 (AC3): Test file hash same for identical content
#[test]
fn test_file_hash_same_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let file1 = temp_dir.path().join("copy1.log");
    let file2 = temp_dir.path().join("copy2.log");

    let content = "identical content in both files\nline 2\nline 3";
    fs::write(&file1, content).expect("Failed to write file1");
    fs::write(&file2, content).expect("Failed to write file2");

    let hash1 = calculate_file_hash(&file1).expect("Hash failed");
    let hash2 = calculate_file_hash(&file2).expect("Hash failed");

    assert_eq!(hash1, hash2, "Identical content should produce same hash");

    eprintln!("[TEST] Identical content produces same hash");
}

/// Story 6.6 (AC3): Test large file hash uses chunked approach
#[test]
fn test_file_hash_large_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("large.log");

    // Create a 5MB file (larger than 2MB threshold for chunked hashing)
    let chunk = vec![b'X'; 1024 * 1024]; // 1MB chunk
    {
        let mut file = File::create(&file_path).expect("Failed to create file");
        for _ in 0..5 {
            file.write_all(&chunk).expect("Failed to write chunk");
        }
    }

    let file_size = fs::metadata(&file_path).expect("Metadata failed").len();
    assert!(file_size >= 5 * 1024 * 1024, "File should be >= 5MB");

    // Hash should still work and be fast (uses first + last 1MB)
    let start = std::time::Instant::now();
    let hash = calculate_file_hash(&file_path).expect("Hash failed");
    let elapsed = start.elapsed();

    assert_eq!(hash.len(), 32, "Hash should be 32 bytes (SHA256)");
    assert!(
        elapsed.as_secs() < 5,
        "Large file hash should complete quickly (took {}s)",
        elapsed.as_secs()
    );

    eprintln!(
        "[TEST] Large file ({} MB) hash completed in {:.2}ms",
        file_size / (1024 * 1024),
        elapsed.as_secs_f64() * 1000.0
    );
}

/// Story 6.6 (AC3): Test cache clear functionality
#[test]
fn test_cache_clear_all() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    // Create multiple cached indexes
    for i in 0..5 {
        let log_path = temp_dir.path().join(format!("log{}.log", i));
        fs::write(&log_path, format!("content for file {}", i)).expect("Failed to write");

        let index = create_test_tiered_index((i + 1) as u64 * 100);
        cache.save_index(&log_path, &index).expect("Failed to save");
    }

    // Verify all cached
    for i in 0..5 {
        let log_path = temp_dir.path().join(format!("log{}.log", i));
        assert!(
            cache.get_cached_index(&log_path).unwrap().is_some(),
            "Log {} should be cached",
            i
        );
    }

    // Clear all
    let cleared = cache.clear_all().expect("Clear failed");
    assert_eq!(cleared, 5, "Should clear 5 cache files");

    // Verify all cleared
    for i in 0..5 {
        let log_path = temp_dir.path().join(format!("log{}.log", i));
        assert!(
            cache.get_cached_index(&log_path).unwrap().is_none(),
            "Log {} should no longer be cached",
            i
        );
    }

    eprintln!("[TEST] Cache clear_all removed {} files", cleared);
}

/// Story 6.6 (AC3): Test cache age tracking
#[test]
fn test_cache_age_tracking() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    let log_path = temp_dir.path().join("age_test.log");
    fs::write(&log_path, "content for age test").expect("Failed to write");

    // No cache yet
    let age_before = cache.get_cache_age(&log_path).expect("Age check failed");
    assert!(age_before.is_none(), "No cache age before saving");

    // Save index
    let index = create_test_tiered_index(50);
    cache.save_index(&log_path, &index).expect("Failed to save");

    // Wait a moment
    thread::sleep(Duration::from_millis(100));

    // Check age
    let age_after = cache.get_cache_age(&log_path).expect("Age check failed");
    assert!(age_after.is_some(), "Should have cache age after saving");

    let age_seconds = age_after.unwrap();
    assert!(
        age_seconds < 10,
        "Cache age should be less than 10 seconds (just created)"
    );

    eprintln!("[TEST] Cache age tracking works: {} seconds", age_seconds);
}

/// Story 6.6 (AC3): Test cache path generation
#[test]
fn test_cache_path_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    // Known hash
    let hash = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
    ];

    let cache_path = cache.get_cache_path(&hash);

    // Should be in index_cache subdirectory with .rkyv extension
    assert!(
        cache_path.to_string_lossy().contains("index_cache"),
        "Cache path should contain 'index_cache'"
    );
    assert!(
        cache_path.extension().map(|e| e == "rkyv").unwrap_or(false),
        "Cache path should have .rkyv extension"
    );

    // Hash should be hex-encoded in filename
    let filename = cache_path.file_stem().unwrap().to_string_lossy();
    assert!(
        filename.contains("123456789abcdef0"),
        "Filename should contain hex-encoded hash: got {}",
        filename
    );

    eprintln!("[TEST] Cache path: {}", cache_path.display());
}

/// Story 6.6 (AC3): Test cache handles missing source file gracefully
#[test]
fn test_cache_missing_source_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = IndexCache::new(temp_dir.path().to_path_buf());

    let log_path = temp_dir.path().join("will_be_deleted.log");
    fs::write(&log_path, "temporary content").expect("Failed to write");

    // Save to cache
    let index = create_test_tiered_index(100);
    cache.save_index(&log_path, &index).expect("Failed to save");

    // Verify hit
    assert!(cache.get_cached_index(&log_path).unwrap().is_some());

    // Delete source file
    fs::remove_file(&log_path).expect("Failed to delete");

    // Cache lookup should return None (source file missing, can't verify hash)
    let result = cache.get_cached_index(&log_path).expect("Lookup failed");
    assert!(
        result.is_none(),
        "Should return None when source file is missing"
    );

    eprintln!("[TEST] Cache handles missing source file gracefully");
}

/// Story 6.6 (AC3): Test concurrent cache operations
#[test]
fn test_cache_concurrent_access() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_dir = temp_dir.path().to_path_buf();

    // Pre-create log files and indexes
    let mut handles = vec![];
    for i in 0..4 {
        let cache_dir_clone = cache_dir.clone();
        let thread_temp = TempDir::new().expect("Failed to create thread temp");
        let log_path = thread_temp.path().join(format!("thread_{}.log", i));
        fs::write(&log_path, format!("content from thread {}", i)).expect("Write failed");

        handles.push(thread::spawn(move || {
            let cache = IndexCache::new(cache_dir_clone);

            // Save
            let index = create_test_tiered_index((i + 1) as u64 * 100);
            cache.save_index(&log_path, &index).expect("Save failed");

            // Multiple reads
            for _ in 0..10 {
                let result = cache.get_cached_index(&log_path).expect("Read failed");
                assert!(result.is_some(), "Thread {} should get cache hit", i);
            }

            // Keep temp dir alive
            drop(thread_temp);
        }));
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    eprintln!("[TEST] Concurrent cache access completed without deadlock");
}
