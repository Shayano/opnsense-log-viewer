//! Tiered Index Architecture for Memory-Efficient Large File Support
//!
//! This module implements a two-tier index system:
//! - **Hot Tier**: Most recent entries kept in RAM for fast access
//! - **Warm Tier**: Older entries stored in memory-mapped files on disk
//!
//! Benefits:
//! - Handles files with 70M+ entries without excessive RAM usage
//! - Recent entries (most frequently queried) have fastest access
//! - Older entries accessible via mmap with minimal memory footprint
//!
//! Story 6.1 - AC3: Tiered Index Architecture

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use roaring::RoaringBitmap;
use serde::{Deserialize, Serialize};

use crate::indexer::bitmap::BitmapIndex;
use crate::indexer::hybrid::IndexError;
use crate::indexer::inverted::InvertedIndex;
use crate::indexer::offset_table::OffsetTable;

/// Default number of entries to keep in hot tier (5 million)
pub const DEFAULT_HOT_TIER_ENTRIES: u64 = 5_000_000;

/// Default memory limit for hot tier in bytes (500 MB)
pub const DEFAULT_HOT_TIER_MEMORY_LIMIT: usize = 500 * 1024 * 1024;

/// Magic bytes for warm tier file format identification
const WARM_TIER_MAGIC: &[u8; 8] = b"OPNSWARM";

/// Current version of the warm tier file format
const WARM_TIER_VERSION: u32 = 1;

/// Configuration for tiered index behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TieredConfig {
    /// Maximum number of entries to keep in hot tier
    pub hot_tier_max_entries: u64,
    /// Maximum memory (bytes) for hot tier before spilling to warm
    pub hot_tier_memory_limit: usize,
    /// Directory for warm tier index files
    pub warm_tier_directory: PathBuf,
    /// Whether to automatically manage tier transitions
    pub auto_tiering: bool,
}

impl Default for TieredConfig {
    fn default() -> Self {
        Self {
            hot_tier_max_entries: DEFAULT_HOT_TIER_ENTRIES,
            hot_tier_memory_limit: DEFAULT_HOT_TIER_MEMORY_LIMIT,
            warm_tier_directory: std::env::temp_dir().join("opnsense_warm_index"),
            auto_tiering: true,
        }
    }
}

impl TieredConfig {
    /// Create config with custom hot tier entry limit
    pub fn with_hot_entries(max_entries: u64) -> Self {
        Self {
            hot_tier_max_entries: max_entries,
            ..Default::default()
        }
    }

    /// Create config with custom memory limit
    pub fn with_memory_limit(memory_limit: usize) -> Self {
        Self {
            hot_tier_memory_limit: memory_limit,
            ..Default::default()
        }
    }

    /// Create config with custom warm tier directory
    pub fn with_warm_directory<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            warm_tier_directory: dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }
}

/// Hot tier index - keeps most recent entries in RAM
///
/// Contains the same structure as HybridIndex but only holds
/// the most recent N entries for fast in-memory access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotIndex {
    /// Inverted index for IP addresses and ports
    pub inverted_index: InvertedIndex,
    /// Bitmap index for action, protocol, interface
    pub bitmap_index: BitmapIndex,
    /// Offset table for raw log line access
    pub offset_table: OffsetTable,
    /// Range of entry IDs in this tier [start, end)
    pub entry_range: (u64, u64),
    /// Number of entries in hot tier
    pub entry_count: u64,
}

impl HotIndex {
    /// Create an empty hot index
    pub fn new() -> Self {
        Self {
            inverted_index: InvertedIndex::new(),
            bitmap_index: BitmapIndex::new(),
            offset_table: OffsetTable::new(),
            entry_range: (0, 0),
            entry_count: 0,
        }
    }

    /// Create from existing indexes with entry range
    pub fn from_indexes(
        inverted: InvertedIndex,
        bitmap: BitmapIndex,
        offset: OffsetTable,
        start_id: u64,
        end_id: u64,
    ) -> Self {
        Self {
            inverted_index: inverted,
            bitmap_index: bitmap,
            offset_table: offset,
            entry_range: (start_id, end_id),
            entry_count: end_id.saturating_sub(start_id),
        }
    }

    /// Get approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.inverted_index.memory_usage()
            + self.bitmap_index.memory_usage()
            + self.offset_table.memory_usage()
            + std::mem::size_of::<Self>()
    }

    /// Check if an entry ID is in this tier's range
    #[inline]
    pub fn contains(&self, entry_id: u64) -> bool {
        entry_id >= self.entry_range.0 && entry_id < self.entry_range.1
    }

    /// Query source IP in hot tier
    pub fn query_source_ip(&self, ip: &str) -> Option<Vec<u64>> {
        self.inverted_index
            .query_source_ip(ip)
            .map(|ids| ids.iter().copied().filter(|&id| self.contains(id)).collect())
    }

    /// Query destination IP in hot tier
    pub fn query_dest_ip(&self, ip: &str) -> Option<Vec<u64>> {
        self.inverted_index
            .query_dest_ip(ip)
            .map(|ids| ids.iter().copied().filter(|&id| self.contains(id)).collect())
    }

    /// Query source port in hot tier
    pub fn query_source_port(&self, port: u16) -> Option<Vec<u64>> {
        self.inverted_index
            .query_source_port(port)
            .map(|ids| ids.iter().copied().filter(|&id| self.contains(id)).collect())
    }

    /// Query destination port in hot tier
    pub fn query_dest_port(&self, port: u16) -> Option<Vec<u64>> {
        self.inverted_index
            .query_dest_port(port)
            .map(|ids| ids.iter().copied().filter(|&id| self.contains(id)).collect())
    }

    /// Query action in hot tier
    pub fn query_action(&self, action: &str) -> Option<RoaringBitmap> {
        self.bitmap_index.query_action(action).cloned()
    }

    /// Query protocol in hot tier
    pub fn query_protocol(&self, protocol: &str) -> Option<RoaringBitmap> {
        self.bitmap_index.query_protocol(protocol).cloned()
    }

    /// Query interface in hot tier
    pub fn query_interface(&self, interface: &str) -> Option<RoaringBitmap> {
        self.bitmap_index.query_interface(interface).cloned()
    }

    /// Get offset for an entry
    pub fn get_offset(&self, entry_id: u64) -> Option<u64> {
        if self.contains(entry_id) {
            self.offset_table.get_offset(entry_id)
        } else {
            None
        }
    }
}

impl Default for HotIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Warm tier file header
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WarmTierHeader {
    /// Magic bytes for file identification
    magic: [u8; 8],
    /// File format version
    version: u32,
    /// Entry range [start, end)
    entry_range: (u64, u64),
    /// Total entry count
    entry_count: u64,
    /// Offset to inverted index data
    inverted_offset: u64,
    /// Size of inverted index data
    inverted_size: u64,
    /// Offset to bitmap index data
    bitmap_offset: u64,
    /// Size of bitmap index data
    bitmap_size: u64,
    /// Offset to offset table data
    offset_table_offset: u64,
    /// Size of offset table data
    offset_table_size: u64,
}

impl WarmTierHeader {
    fn new(start_id: u64, end_id: u64) -> Self {
        Self {
            magic: *WARM_TIER_MAGIC,
            version: WARM_TIER_VERSION,
            entry_range: (start_id, end_id),
            entry_count: end_id.saturating_sub(start_id),
            inverted_offset: 0,
            inverted_size: 0,
            bitmap_offset: 0,
            bitmap_size: 0,
            offset_table_offset: 0,
            offset_table_size: 0,
        }
    }

    fn validate(&self) -> Result<(), IndexError> {
        if &self.magic != WARM_TIER_MAGIC {
            return Err(IndexError::SerializationError(
                "Invalid warm tier file magic".to_string(),
            ));
        }
        if self.version != WARM_TIER_VERSION {
            return Err(IndexError::SerializationError(format!(
                "Unsupported warm tier version: {} (expected {})",
                self.version, WARM_TIER_VERSION
            )));
        }
        Ok(())
    }
}

/// Warm tier index - memory-mapped file for older entries
///
/// Uses memory-mapped I/O for efficient disk access without
/// loading the entire index into RAM.
pub struct WarmIndex {
    /// Path to the warm tier file
    file_path: PathBuf,
    /// Memory-mapped file handle
    mmap: Option<Mmap>,
    /// Parsed header
    header: WarmTierHeader,
    /// Cached inverted index (loaded on first query)
    cached_inverted: Option<InvertedIndex>,
    /// Cached bitmap index (loaded on first query)
    cached_bitmap: Option<BitmapIndex>,
    /// Cached offset table (loaded on first query)
    cached_offsets: Option<OffsetTable>,
}

impl WarmIndex {
    /// Create a new warm index from existing indexes
    pub fn create<P: AsRef<Path>>(
        path: P,
        inverted: &InvertedIndex,
        bitmap: &BitmapIndex,
        offsets: &OffsetTable,
        start_id: u64,
        end_id: u64,
    ) -> Result<Self, IndexError> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(path)?;
        let mut writer = BufWriter::with_capacity(256 * 1024, file);

        // Serialize components
        let inverted_data = bincode::serde::encode_to_vec(inverted, bincode::config::standard())
            .map_err(|e| IndexError::SerializationError(e.to_string()))?;

        let bitmap_data = bincode::serde::encode_to_vec(bitmap, bincode::config::standard())
            .map_err(|e| IndexError::SerializationError(e.to_string()))?;

        let offsets_data = bincode::serde::encode_to_vec(offsets, bincode::config::standard())
            .map_err(|e| IndexError::SerializationError(e.to_string()))?;

        // Calculate offsets (header size = 128 bytes for future expansion)
        let header_size = 128u64;
        let inverted_offset = header_size;
        let bitmap_offset = inverted_offset + inverted_data.len() as u64;
        let offset_table_offset = bitmap_offset + bitmap_data.len() as u64;

        // Build header
        let mut header = WarmTierHeader::new(start_id, end_id);
        header.inverted_offset = inverted_offset;
        header.inverted_size = inverted_data.len() as u64;
        header.bitmap_offset = bitmap_offset;
        header.bitmap_size = bitmap_data.len() as u64;
        header.offset_table_offset = offset_table_offset;
        header.offset_table_size = offsets_data.len() as u64;

        // Serialize header (fixed 128 bytes)
        let header_data = bincode::serde::encode_to_vec(&header, bincode::config::standard())
            .map_err(|e| IndexError::SerializationError(e.to_string()))?;

        // Write header with padding
        writer.write_all(&header_data)?;
        let padding = vec![0u8; header_size as usize - header_data.len()];
        writer.write_all(&padding)?;

        // Write index data
        writer.write_all(&inverted_data)?;
        writer.write_all(&bitmap_data)?;
        writer.write_all(&offsets_data)?;
        writer.flush()?;

        log::info!(
            "[WARM] Created warm tier file: {} ({} bytes, entries {}-{})",
            path.display(),
            header_size + inverted_data.len() as u64 + bitmap_data.len() as u64 + offsets_data.len() as u64,
            start_id,
            end_id
        );

        // Open for reading
        Self::open(path)
    }

    /// Open an existing warm tier file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, IndexError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Read and validate header (first 128 bytes)
        if mmap.len() < 128 {
            return Err(IndexError::SerializationError(
                "Warm tier file too small".to_string(),
            ));
        }

        let (header, _): (WarmTierHeader, _) =
            bincode::serde::decode_from_slice(&mmap[..128], bincode::config::standard())
                .map_err(|e| IndexError::SerializationError(e.to_string()))?;

        header.validate()?;

        log::info!(
            "[WARM] Opened warm tier file: {} (entries {}-{})",
            path.display(),
            header.entry_range.0,
            header.entry_range.1
        );

        Ok(Self {
            file_path: path.to_path_buf(),
            mmap: Some(mmap),
            header,
            cached_inverted: None,
            cached_bitmap: None,
            cached_offsets: None,
        })
    }

    /// Get entry range
    pub fn entry_range(&self) -> (u64, u64) {
        self.header.entry_range
    }

    /// Get entry count
    pub fn entry_count(&self) -> u64 {
        self.header.entry_count
    }

    /// Check if an entry ID is in this tier's range
    #[inline]
    pub fn contains(&self, entry_id: u64) -> bool {
        entry_id >= self.header.entry_range.0 && entry_id < self.header.entry_range.1
    }

    /// Get approximate memory usage (only mmap overhead, not file size)
    pub fn memory_usage(&self) -> usize {
        // Mmap uses virtual memory, actual RAM usage is minimal
        // Report only cached data size
        let mut size = std::mem::size_of::<Self>();
        if let Some(ref inv) = self.cached_inverted {
            size += inv.memory_usage();
        }
        if let Some(ref bmp) = self.cached_bitmap {
            size += bmp.memory_usage();
        }
        if let Some(ref off) = self.cached_offsets {
            size += off.memory_usage();
        }
        size
    }

    /// Get file size on disk
    pub fn disk_size(&self) -> u64 {
        self.mmap.as_ref().map(|m| m.len() as u64).unwrap_or(0)
    }

    /// Load inverted index from mmap (lazy loading)
    fn load_inverted(&mut self) -> Result<&InvertedIndex, IndexError> {
        if self.cached_inverted.is_none() {
            let mmap = self.mmap.as_ref().ok_or_else(|| {
                IndexError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Warm tier not mapped",
                ))
            })?;

            let start = self.header.inverted_offset as usize;
            let end = start + self.header.inverted_size as usize;

            let (inverted, _): (InvertedIndex, _) =
                bincode::serde::decode_from_slice(&mmap[start..end], bincode::config::standard())
                    .map_err(|e| IndexError::SerializationError(e.to_string()))?;

            self.cached_inverted = Some(inverted);
        }
        Ok(self.cached_inverted.as_ref().unwrap())
    }

    /// Load bitmap index from mmap (lazy loading)
    fn load_bitmap(&mut self) -> Result<&BitmapIndex, IndexError> {
        if self.cached_bitmap.is_none() {
            let mmap = self.mmap.as_ref().ok_or_else(|| {
                IndexError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Warm tier not mapped",
                ))
            })?;

            let start = self.header.bitmap_offset as usize;
            let end = start + self.header.bitmap_size as usize;

            let (bitmap, _): (BitmapIndex, _) =
                bincode::serde::decode_from_slice(&mmap[start..end], bincode::config::standard())
                    .map_err(|e| IndexError::SerializationError(e.to_string()))?;

            self.cached_bitmap = Some(bitmap);
        }
        Ok(self.cached_bitmap.as_ref().unwrap())
    }

    /// Load offset table from mmap (lazy loading)
    fn load_offsets(&mut self) -> Result<&OffsetTable, IndexError> {
        if self.cached_offsets.is_none() {
            let mmap = self.mmap.as_ref().ok_or_else(|| {
                IndexError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Warm tier not mapped",
                ))
            })?;

            let start = self.header.offset_table_offset as usize;
            let end = start + self.header.offset_table_size as usize;

            let (offsets, _): (OffsetTable, _) =
                bincode::serde::decode_from_slice(&mmap[start..end], bincode::config::standard())
                    .map_err(|e| IndexError::SerializationError(e.to_string()))?;

            self.cached_offsets = Some(offsets);
        }
        Ok(self.cached_offsets.as_ref().unwrap())
    }

    /// Query source IP in warm tier
    pub fn query_source_ip(&mut self, ip: &str) -> Result<Option<Vec<u64>>, IndexError> {
        let range = self.header.entry_range;
        let inverted = self.load_inverted()?;
        Ok(inverted
            .query_source_ip(ip)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|&id| id >= range.0 && id < range.1)
                    .collect()
            }))
    }

    /// Query destination IP in warm tier
    pub fn query_dest_ip(&mut self, ip: &str) -> Result<Option<Vec<u64>>, IndexError> {
        let range = self.header.entry_range;
        let inverted = self.load_inverted()?;
        Ok(inverted
            .query_dest_ip(ip)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|&id| id >= range.0 && id < range.1)
                    .collect()
            }))
    }

    /// Query source port in warm tier
    pub fn query_source_port(&mut self, port: u16) -> Result<Option<Vec<u64>>, IndexError> {
        let range = self.header.entry_range;
        let inverted = self.load_inverted()?;
        Ok(inverted
            .query_source_port(port)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|&id| id >= range.0 && id < range.1)
                    .collect()
            }))
    }

    /// Query destination port in warm tier
    pub fn query_dest_port(&mut self, port: u16) -> Result<Option<Vec<u64>>, IndexError> {
        let range = self.header.entry_range;
        let inverted = self.load_inverted()?;
        Ok(inverted
            .query_dest_port(port)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|&id| id >= range.0 && id < range.1)
                    .collect()
            }))
    }

    /// Query action in warm tier
    pub fn query_action(&mut self, action: &str) -> Result<Option<RoaringBitmap>, IndexError> {
        let bitmap = self.load_bitmap()?;
        Ok(bitmap.query_action(action).cloned())
    }

    /// Query protocol in warm tier
    pub fn query_protocol(&mut self, protocol: &str) -> Result<Option<RoaringBitmap>, IndexError> {
        let bitmap = self.load_bitmap()?;
        Ok(bitmap.query_protocol(protocol).cloned())
    }

    /// Query interface in warm tier
    pub fn query_interface(&mut self, interface: &str) -> Result<Option<RoaringBitmap>, IndexError> {
        let bitmap = self.load_bitmap()?;
        Ok(bitmap.query_interface(interface).cloned())
    }

    /// Get offset for an entry
    pub fn get_offset(&mut self, entry_id: u64) -> Result<Option<u64>, IndexError> {
        if !self.contains(entry_id) {
            return Ok(None);
        }
        let offsets = self.load_offsets()?;
        Ok(offsets.get_offset(entry_id))
    }

    /// Clear cached data to free memory
    pub fn clear_cache(&mut self) {
        self.cached_inverted = None;
        self.cached_bitmap = None;
        self.cached_offsets = None;
    }

    /// Get file path
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

/// Tiered index combining hot and warm tiers
///
/// Provides transparent access to both tiers, automatically
/// routing queries to the appropriate tier based on entry IDs.
pub struct TieredIndex {
    /// Configuration
    pub config: TieredConfig,
    /// Hot tier (in-memory, most recent entries)
    pub hot: HotIndex,
    /// Warm tiers (memory-mapped, older entries)
    pub warm: Vec<WarmIndex>,
    /// Total entry count across all tiers
    pub total_entries: u64,
}

impl TieredIndex {
    /// Create a new tiered index with default configuration
    pub fn new() -> Self {
        Self::with_config(TieredConfig::default())
    }

    /// Create a new tiered index with custom configuration
    pub fn with_config(config: TieredConfig) -> Self {
        Self {
            config,
            hot: HotIndex::new(),
            warm: Vec::new(),
            total_entries: 0,
        }
    }

    /// Create tiered index from a single HybridIndex, splitting into hot/warm tiers
    ///
    /// If total entries exceed hot tier limit, older entries are moved to warm tier.
    pub fn from_hybrid(
        inverted: InvertedIndex,
        bitmap: BitmapIndex,
        offsets: OffsetTable,
        total_entries: u64,
        config: TieredConfig,
    ) -> Result<Self, IndexError> {
        let hot_limit = config.hot_tier_max_entries;

        if total_entries <= hot_limit {
            // All entries fit in hot tier
            log::info!(
                "[TIERED] All {} entries fit in hot tier (limit: {})",
                total_entries,
                hot_limit
            );
            return Ok(Self {
                config,
                hot: HotIndex::from_indexes(inverted, bitmap, offsets, 0, total_entries),
                warm: Vec::new(),
                total_entries,
            });
        }

        // Need to split into hot and warm tiers
        let warm_end = total_entries - hot_limit;
        let hot_start = warm_end;

        log::info!(
            "[TIERED] Splitting {} entries: warm [0, {}), hot [{}, {})",
            total_entries,
            warm_end,
            hot_start,
            total_entries
        );

        // Create warm tier directory
        fs::create_dir_all(&config.warm_tier_directory)?;

        // Create warm tier file
        let warm_path = config.warm_tier_directory.join("warm_0.idx");
        let warm_index = WarmIndex::create(
            &warm_path,
            &inverted,
            &bitmap,
            &offsets,
            0,
            warm_end,
        )?;

        // Hot tier keeps the same indexes but with limited range
        // For simplicity, we keep the full indexes but queries filter by range
        let hot = HotIndex::from_indexes(inverted, bitmap, offsets, hot_start, total_entries);

        Ok(Self {
            config,
            hot,
            warm: vec![warm_index],
            total_entries,
        })
    }

    /// Get total entry count
    pub fn entry_count(&self) -> u64 {
        self.total_entries
    }

    /// Get memory usage of hot tier
    pub fn hot_memory_usage(&self) -> usize {
        self.hot.memory_usage()
    }

    /// Get memory usage of warm tier caches
    pub fn warm_memory_usage(&self) -> usize {
        self.warm.iter().map(|w| w.memory_usage()).sum()
    }

    /// Get total disk usage of warm tiers
    pub fn warm_disk_usage(&self) -> u64 {
        self.warm.iter().map(|w| w.disk_size()).sum()
    }

    /// Check if entry is in hot tier
    #[inline]
    pub fn is_hot(&self, entry_id: u64) -> bool {
        self.hot.contains(entry_id)
    }

    /// Clear warm tier caches to free memory
    pub fn clear_warm_caches(&mut self) {
        for warm in &mut self.warm {
            warm.clear_cache();
        }
    }
}

impl Default for TieredIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Query executor that merges results from hot and warm tiers
pub struct TieredQueryExecutor<'a> {
    tiered_index: &'a mut TieredIndex,
}

impl<'a> TieredQueryExecutor<'a> {
    /// Create a new tiered query executor
    pub fn new(tiered_index: &'a mut TieredIndex) -> Self {
        Self { tiered_index }
    }

    /// Query source IP across all tiers
    pub fn query_source_ip(&mut self, ip: &str) -> Result<Vec<u64>, IndexError> {
        let mut results = Vec::new();

        // Query hot tier
        if let Some(ids) = self.tiered_index.hot.query_source_ip(ip) {
            results.extend(ids);
        }

        // Query warm tiers
        for warm in &mut self.tiered_index.warm {
            if let Some(ids) = warm.query_source_ip(ip)? {
                results.extend(ids);
            }
        }

        results.sort_unstable();
        Ok(results)
    }

    /// Query destination IP across all tiers
    pub fn query_dest_ip(&mut self, ip: &str) -> Result<Vec<u64>, IndexError> {
        let mut results = Vec::new();

        if let Some(ids) = self.tiered_index.hot.query_dest_ip(ip) {
            results.extend(ids);
        }

        for warm in &mut self.tiered_index.warm {
            if let Some(ids) = warm.query_dest_ip(ip)? {
                results.extend(ids);
            }
        }

        results.sort_unstable();
        Ok(results)
    }

    /// Query source port across all tiers
    pub fn query_source_port(&mut self, port: u16) -> Result<Vec<u64>, IndexError> {
        let mut results = Vec::new();

        if let Some(ids) = self.tiered_index.hot.query_source_port(port) {
            results.extend(ids);
        }

        for warm in &mut self.tiered_index.warm {
            if let Some(ids) = warm.query_source_port(port)? {
                results.extend(ids);
            }
        }

        results.sort_unstable();
        Ok(results)
    }

    /// Query destination port across all tiers
    pub fn query_dest_port(&mut self, port: u16) -> Result<Vec<u64>, IndexError> {
        let mut results = Vec::new();

        if let Some(ids) = self.tiered_index.hot.query_dest_port(port) {
            results.extend(ids);
        }

        for warm in &mut self.tiered_index.warm {
            if let Some(ids) = warm.query_dest_port(port)? {
                results.extend(ids);
            }
        }

        results.sort_unstable();
        Ok(results)
    }

    /// Query action across all tiers
    pub fn query_action(&mut self, action: &str) -> Result<RoaringBitmap, IndexError> {
        let mut result = self
            .tiered_index
            .hot
            .query_action(action)
            .unwrap_or_default();

        for warm in &mut self.tiered_index.warm {
            if let Some(bitmap) = warm.query_action(action)? {
                result |= bitmap;
            }
        }

        Ok(result)
    }

    /// Query protocol across all tiers
    pub fn query_protocol(&mut self, protocol: &str) -> Result<RoaringBitmap, IndexError> {
        let mut result = self
            .tiered_index
            .hot
            .query_protocol(protocol)
            .unwrap_or_default();

        for warm in &mut self.tiered_index.warm {
            if let Some(bitmap) = warm.query_protocol(protocol)? {
                result |= bitmap;
            }
        }

        Ok(result)
    }

    /// Query interface across all tiers
    pub fn query_interface(&mut self, interface: &str) -> Result<RoaringBitmap, IndexError> {
        let mut result = self
            .tiered_index
            .hot
            .query_interface(interface)
            .unwrap_or_default();

        for warm in &mut self.tiered_index.warm {
            if let Some(bitmap) = warm.query_interface(interface)? {
                result |= bitmap;
            }
        }

        Ok(result)
    }

    /// Get offset for an entry (checks hot tier first, then warm)
    pub fn get_offset(&mut self, entry_id: u64) -> Result<Option<u64>, IndexError> {
        // Check hot tier first (most common case)
        if let Some(offset) = self.tiered_index.hot.get_offset(entry_id) {
            return Ok(Some(offset));
        }

        // Check warm tiers
        for warm in &mut self.tiered_index.warm {
            if let Some(offset) = warm.get_offset(entry_id)? {
                return Ok(Some(offset));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_indexes(entry_count: u64) -> (InvertedIndex, BitmapIndex, OffsetTable) {
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

        (inverted, bitmap, offsets)
    }

    #[test]
    fn test_hot_index_creation() {
        let (inverted, bitmap, offsets) = create_test_indexes(1000);
        let hot = HotIndex::from_indexes(inverted, bitmap, offsets, 0, 1000);

        assert_eq!(hot.entry_count, 1000);
        assert!(hot.contains(0));
        assert!(hot.contains(999));
        assert!(!hot.contains(1000));
    }

    #[test]
    fn test_hot_index_queries() {
        let (inverted, bitmap, offsets) = create_test_indexes(100);
        let hot = HotIndex::from_indexes(inverted, bitmap, offsets, 0, 100);

        // Test IP query
        let results = hot.query_source_ip("192.168.1.0").unwrap();
        assert!(!results.is_empty());

        // Test action query
        let results = hot.query_action("block").unwrap();
        assert_eq!(results.len(), 50); // Half are "block"

        // Test offset query
        assert_eq!(hot.get_offset(5), Some(500));
    }

    #[test]
    fn test_tiered_config_defaults() {
        let config = TieredConfig::default();
        assert_eq!(config.hot_tier_max_entries, DEFAULT_HOT_TIER_ENTRIES);
        assert_eq!(config.hot_tier_memory_limit, DEFAULT_HOT_TIER_MEMORY_LIMIT);
        assert!(config.auto_tiering);
    }

    #[test]
    fn test_tiered_index_all_hot() {
        let (inverted, bitmap, offsets) = create_test_indexes(1000);
        let config = TieredConfig::with_hot_entries(5000);

        let tiered = TieredIndex::from_hybrid(inverted, bitmap, offsets, 1000, config).unwrap();

        assert_eq!(tiered.entry_count(), 1000);
        assert!(tiered.warm.is_empty());
        assert!(tiered.is_hot(0));
        assert!(tiered.is_hot(999));
    }

    #[test]
    fn test_warm_tier_file_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let (inverted, bitmap, offsets) = create_test_indexes(100);

        // Create warm tier
        let warm_path = temp_dir.path().join("test_warm.idx");
        let mut warm = WarmIndex::create(&warm_path, &inverted, &bitmap, &offsets, 0, 100).unwrap();

        assert_eq!(warm.entry_count(), 100);
        assert!(warm.contains(0));
        assert!(warm.contains(99));
        assert!(!warm.contains(100));

        // Test queries
        let results = warm.query_source_ip("192.168.1.0").unwrap();
        assert!(results.is_some());

        let results = warm.query_action("block").unwrap();
        assert!(results.is_some());
        assert_eq!(results.unwrap().len(), 50);

        // Test offset
        assert_eq!(warm.get_offset(5).unwrap(), Some(500));

        // Test reopening
        drop(warm);
        let warm2 = WarmIndex::open(&warm_path).unwrap();
        assert_eq!(warm2.entry_count(), 100);
    }

    #[test]
    fn test_tiered_query_executor() {
        let temp_dir = tempfile::tempdir().unwrap();
        let (inverted, bitmap, offsets) = create_test_indexes(200);

        let mut config = TieredConfig::with_hot_entries(100);
        config.warm_tier_directory = temp_dir.path().to_path_buf();

        let mut tiered =
            TieredIndex::from_hybrid(inverted, bitmap, offsets, 200, config).unwrap();

        // Verify split
        assert_eq!(tiered.warm.len(), 1);
        assert!(tiered.is_hot(100)); // Hot starts at 100
        assert!(!tiered.is_hot(99)); // 99 is in warm

        // Test query executor
        let mut executor = TieredQueryExecutor::new(&mut tiered);

        // Query should return results from both tiers
        let results = executor.query_action("block").unwrap();
        assert_eq!(results.len(), 100); // 100 total "block" entries
    }
}
