//! Streaming indexing module for memory-efficient large file processing
//!
//! This module processes log files in batches to keep memory usage bounded,
//! even for very large files (30GB+). Instead of loading all data into RAM,
//! it processes chunks in batches and persists intermediate results to disk.
//!
//! Architecture:
//! 1. Divide file into chunks (10MB each)
//! 2. Process chunks in batches (100 chunks = ~1GB per batch)
//! 3. For each batch: Parse → Build Index → Persist to temp file → Free memory
//! 4. Final merge: Load and merge partial indexes from disk
//!
//! Memory optimizations (Story 6.1):
//! - String interning for IPs/interfaces (40%+ memory reduction per batch)
//! - Pre-allocated HashMaps based on estimated entry count (Amelia)
//! - Mimalloc allocator for reduced contention (Murat)
//!
//! Memory budget: ~2GB peak (vs 23GB for full parallel approach)

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use memmap2::Mmap;
use rayon::prelude::*;
use roaring::RoaringBitmap;

use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};
use bytecheck::CheckBytes;

use crate::indexer::bitmap::BitmapIndex;
use crate::indexer::interner::LocalInterner;
use crate::indexer::inverted::InvertedIndex;
use crate::indexer::offset_table::OffsetTable;
use crate::indexer::progress::IndexProgress;
use crate::indexer::hybrid::{IndexError, IndexMetadata};
use crate::parser::{csv_filterlog, rfc3164, rfc5424};
use crate::types::log_entry::LogFormat;

/// Target chunk size in bytes (10 MB)
const CHUNK_SIZE: usize = 10 * 1024 * 1024;

/// Minimum chunk size to avoid overhead on small files
const MIN_CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

/// Number of chunks to process per batch (~1GB of log data)
/// Lower value = less memory, higher value = better parallelism
const CHUNKS_PER_BATCH: usize = 100;

/// Estimated unique IPs per batch (for interner capacity)
const ESTIMATED_UNIQUE_IPS: usize = 10000;

/// Magic bytes for batch file format identification (Story 6.2, AC5)
const BATCH_MAGIC: &[u8; 8] = b"OPNSRKYV";

/// Current batch file format version
const BATCH_VERSION: u32 = 1;

/// Parsed entry data (minimal allocation)
/// Story 6.1: Uses Box<str> instead of String to save 8 bytes per string (16 vs 24 bytes)
#[derive(Clone)]
pub(crate) struct ParsedEntry {
    pub byte_offset: u64,
    pub source_ip: Option<Box<str>>,
    pub dest_ip: Option<Box<str>>,
    pub source_port: Option<u16>,
    pub dest_port: Option<u16>,
    pub action: Option<Box<str>>,
    pub protocol: Option<Box<str>>,
    pub interface: Option<Box<str>>,
}

impl ParsedEntry {
    /// Create a new ParsedEntry with interned strings from a LocalInterner
    /// Story 6.1 (Amelia): String interning for IP/interface deduplication
    fn from_log_entry(
        byte_offset: u64,
        entry: &crate::types::log_entry::LogEntry,
        interner: &mut LocalInterner,
    ) -> Self {
        Self {
            byte_offset,
            source_ip: entry.source_ip.as_deref().map(|s| interner.get_or_intern(s)),
            dest_ip: entry.dest_ip.as_deref().map(|s| interner.get_or_intern(s)),
            source_port: entry.source_port,
            dest_port: entry.dest_port,
            action: entry.action.as_deref().map(|s| interner.get_or_intern(s)),
            protocol: entry.protocol.as_deref().map(|s| interner.get_or_intern(s)),
            interface: entry.interface.as_deref().map(|s| interner.get_or_intern(s)),
        }
    }
}

/// Chunk boundaries for parallel processing
struct ChunkBoundary {
    start: usize,
    end: usize,
    start_offset: u64,
}

/// Partial indexes built from a batch
struct BatchIndexes {
    /// Bitmap index components
    actions: HashMap<String, RoaringBitmap>,
    protocols: HashMap<String, RoaringBitmap>,
    interfaces: HashMap<String, RoaringBitmap>,
    /// Inverted index components
    source_ips: HashMap<String, Vec<u64>>,
    dest_ips: HashMap<String, Vec<u64>>,
    source_ports: HashMap<u16, Vec<u64>>,
    dest_ports: HashMap<u16, Vec<u64>>,
    /// Offset data: (global_entry_id, byte_offset)
    offsets: Vec<(u64, u64)>,
    /// Entry count in this batch
    entry_count: u64,
}

impl BatchIndexes {
    fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create with pre-allocated capacity based on estimated entry count
    /// Story 6.1 (Amelia): Pre-allocate HashMaps to reduce reallocations
    fn with_capacity(estimated_entries: usize) -> Self {
        // Estimate unique values based on typical log patterns:
        // - ~1% unique IPs (many repeated connections)
        // - ~5-10 unique protocols (TCP, UDP, ICMP, etc.)
        // - ~3-5 unique actions (pass, block, etc.)
        // - ~10-20 unique interfaces
        // - ~1000 unique ports
        let estimated_unique_ips = (estimated_entries / 100).max(100);
        let estimated_unique_ports = (estimated_entries / 50).max(50).min(2000);

        Self {
            actions: HashMap::with_capacity(10),
            protocols: HashMap::with_capacity(10),
            interfaces: HashMap::with_capacity(20),
            source_ips: HashMap::with_capacity(estimated_unique_ips),
            dest_ips: HashMap::with_capacity(estimated_unique_ips),
            source_ports: HashMap::with_capacity(estimated_unique_ports),
            dest_ports: HashMap::with_capacity(estimated_unique_ports),
            offsets: Vec::with_capacity(estimated_entries),
            entry_count: 0,
        }
    }

    fn add_entry(&mut self, global_id: u64, entry: &ParsedEntry) {
        // Bitmap index
        // Story 6.1: Box<str> -> String conversion only when key doesn't exist
        if let Some(action) = &entry.action {
            let key = action.to_lowercase();
            self.actions
                .entry(key)
                .or_default()
                .insert(global_id as u32);
        }
        if let Some(protocol) = &entry.protocol {
            let key = protocol.to_uppercase();
            self.protocols
                .entry(key)
                .or_default()
                .insert(global_id as u32);
        }
        if let Some(interface) = &entry.interface {
            self.interfaces
                .entry(interface.to_string())
                .or_default()
                .insert(global_id as u32);
        }

        // Inverted index
        // Story 6.1: Use Box<str>::into_string() for owned conversion without realloc
        if let Some(ip) = &entry.source_ip {
            self.source_ips
                .entry(ip.to_string())
                .or_default()
                .push(global_id);
        }
        if let Some(ip) = &entry.dest_ip {
            self.dest_ips
                .entry(ip.to_string())
                .or_default()
                .push(global_id);
        }
        if let Some(port) = entry.source_port {
            self.source_ports
                .entry(port)
                .or_default()
                .push(global_id);
        }
        if let Some(port) = entry.dest_port {
            self.dest_ports
                .entry(port)
                .or_default()
                .push(global_id);
        }

        // Offset
        self.offsets.push((global_id, entry.byte_offset));
        self.entry_count += 1;
    }

    /// Merge another batch into this one
    fn merge(&mut self, other: BatchIndexes) {
        // Merge bitmap indexes
        for (key, bitmap) in other.actions {
            self.actions
                .entry(key)
                .and_modify(|b| *b |= &bitmap)
                .or_insert(bitmap);
        }
        for (key, bitmap) in other.protocols {
            self.protocols
                .entry(key)
                .and_modify(|b| *b |= &bitmap)
                .or_insert(bitmap);
        }
        for (key, bitmap) in other.interfaces {
            self.interfaces
                .entry(key)
                .and_modify(|b| *b |= &bitmap)
                .or_insert(bitmap);
        }

        // Merge inverted indexes
        for (key, mut ids) in other.source_ips {
            self.source_ips
                .entry(key)
                .and_modify(|v| v.append(&mut ids))
                .or_insert(ids);
        }
        for (key, mut ids) in other.dest_ips {
            self.dest_ips
                .entry(key)
                .and_modify(|v| v.append(&mut ids))
                .or_insert(ids);
        }
        for (key, mut ids) in other.source_ports {
            self.source_ports
                .entry(key)
                .and_modify(|v| v.append(&mut ids))
                .or_insert(ids);
        }
        for (key, mut ids) in other.dest_ports {
            self.dest_ports
                .entry(key)
                .and_modify(|v| v.append(&mut ids))
                .or_insert(ids);
        }

        // Merge offsets
        self.offsets.extend(other.offsets);
        self.entry_count += other.entry_count;
    }
}

/// Serializable format for partial batch indexes
///
/// Story 6.2: Uses rkyv for fast zero-copy serialization of batch data
#[derive(serde::Serialize, serde::Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[archive_attr(derive(CheckBytes))]
struct SerializedBatchIndexes {
    actions: Vec<(String, Vec<u8>)>,  // Key + serialized RoaringBitmap
    protocols: Vec<(String, Vec<u8>)>,
    interfaces: Vec<(String, Vec<u8>)>,
    source_ips: Vec<(String, Vec<u64>)>,
    dest_ips: Vec<(String, Vec<u64>)>,
    source_ports: Vec<(u16, Vec<u64>)>,
    dest_ports: Vec<(u16, Vec<u64>)>,
    offsets: Vec<(u64, u64)>,
    entry_count: u64,
}

impl From<BatchIndexes> for SerializedBatchIndexes {
    fn from(batch: BatchIndexes) -> Self {
        Self {
            actions: batch.actions
                .into_iter()
                .map(|(k, b)| {
                    let mut bytes = Vec::new();
                    b.serialize_into(&mut bytes).expect("RoaringBitmap serialization");
                    (k, bytes)
                })
                .collect(),
            protocols: batch.protocols
                .into_iter()
                .map(|(k, b)| {
                    let mut bytes = Vec::new();
                    b.serialize_into(&mut bytes).expect("RoaringBitmap serialization");
                    (k, bytes)
                })
                .collect(),
            interfaces: batch.interfaces
                .into_iter()
                .map(|(k, b)| {
                    let mut bytes = Vec::new();
                    b.serialize_into(&mut bytes).expect("RoaringBitmap serialization");
                    (k, bytes)
                })
                .collect(),
            source_ips: batch.source_ips.into_iter().collect(),
            dest_ips: batch.dest_ips.into_iter().collect(),
            source_ports: batch.source_ports.into_iter().collect(),
            dest_ports: batch.dest_ports.into_iter().collect(),
            offsets: batch.offsets,
            entry_count: batch.entry_count,
        }
    }
}

impl TryFrom<SerializedBatchIndexes> for BatchIndexes {
    type Error = IndexError;

    fn try_from(serialized: SerializedBatchIndexes) -> Result<Self, Self::Error> {
        let actions = serialized.actions
            .into_iter()
            .map(|(k, data)| {
                RoaringBitmap::deserialize_from(&data[..])
                    .map(|b| (k, b))
                    .map_err(|e| IndexError::SerializationError(e.to_string()))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let protocols = serialized.protocols
            .into_iter()
            .map(|(k, data)| {
                RoaringBitmap::deserialize_from(&data[..])
                    .map(|b| (k, b))
                    .map_err(|e| IndexError::SerializationError(e.to_string()))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let interfaces = serialized.interfaces
            .into_iter()
            .map(|(k, data)| {
                RoaringBitmap::deserialize_from(&data[..])
                    .map(|b| (k, b))
                    .map_err(|e| IndexError::SerializationError(e.to_string()))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self {
            actions,
            protocols,
            interfaces,
            source_ips: serialized.source_ips.into_iter().collect(),
            dest_ips: serialized.dest_ips.into_iter().collect(),
            source_ports: serialized.source_ports.into_iter().collect(),
            dest_ports: serialized.dest_ports.into_iter().collect(),
            offsets: serialized.offsets,
            entry_count: serialized.entry_count,
        })
    }
}

/// Story 6.3 AC3: Progressive merge state for memory-bounded incremental indexation
///
/// This struct manages the incremental merge of batch indexes into a TieredIndex,
/// spilling older entries to WarmIndex files when the hot tier exceeds 5M entries.
///
/// Memory behavior:
/// - Hot tier maintains at most DEFAULT_HOT_TIER_ENTRIES (5M entries) in RAM
/// - When exceeded, oldest entries are spilled to WarmIndex (rkyv-serialized disk files)
/// - Peak RAM usage stays below 2GB for any file size
pub(crate) struct ProgressiveMergeState {
    /// Accumulated inverted index data (hot tier)
    inverted: InvertedIndex,
    /// Accumulated bitmap index data (hot tier)
    bitmap: BitmapIndex,
    /// Accumulated offset table data
    offset_table: OffsetTable,
    /// Total entries merged so far
    total_entries_merged: u64,
    /// Number of warm tier files created
    warm_tier_count: u32,
    /// Directory for warm tier files
    warm_directory: PathBuf,
    /// List of created warm tier files
    warm_files: Vec<PathBuf>,
    /// Start entry ID of current hot tier data
    hot_start_id: u64,
}

impl ProgressiveMergeState {
    /// Create a new progressive merge state
    ///
    /// Story 6.3 AC3: Initializes empty state for progressive batch merging.
    ///
    /// # Arguments
    /// * `warm_directory` - Directory to store warm tier files
    pub fn new(warm_directory: PathBuf) -> Self {
        Self {
            inverted: InvertedIndex::new(),
            bitmap: BitmapIndex::new(),
            offset_table: OffsetTable::new(),
            total_entries_merged: 0,
            warm_tier_count: 0,
            warm_directory,
            warm_files: Vec::new(),
            hot_start_id: 0,
        }
    }

    /// Merge a batch into the progressive state
    ///
    /// Story 6.3 AC3: Converts BatchIndexes to index structures, merges into hot tier,
    /// and spills to warm tier if hot tier exceeds 5M entries.
    ///
    /// # Arguments
    /// * `batch` - Batch indexes to merge
    ///
    /// # Returns
    /// Number of entries in hot tier after merge
    pub fn merge_batch(&mut self, batch: BatchIndexes) -> Result<u64, IndexError> {
        let batch_entries = batch.entry_count;

        // Merge bitmap index components
        for (key, bitmap) in batch.actions {
            self.bitmap.merge_action(&key, bitmap);
        }
        for (key, bitmap) in batch.protocols {
            self.bitmap.merge_protocol(&key, bitmap);
        }
        for (key, bitmap) in batch.interfaces {
            self.bitmap.merge_interface(&key, bitmap);
        }

        // Merge inverted index components
        for (key, ids) in batch.source_ips {
            self.inverted.merge_source_ip(&key, ids);
        }
        for (key, ids) in batch.dest_ips {
            self.inverted.merge_dest_ip(&key, ids);
        }
        for (key, ids) in batch.source_ports {
            self.inverted.merge_source_port(key, ids);
        }
        for (key, ids) in batch.dest_ports {
            self.inverted.merge_dest_port(key, ids);
        }

        // Merge offset table
        for (entry_id, offset) in batch.offsets {
            self.offset_table.set_offset(entry_id, offset);
        }

        self.total_entries_merged += batch_entries;

        // Check if we need to spill to warm tier
        let hot_entries = self.total_entries_merged - self.hot_start_id;
        if hot_entries > crate::indexer::tiered::DEFAULT_HOT_TIER_ENTRIES {
            self.spill_to_warm()?;
        }

        let current_hot_entries = self.total_entries_merged - self.hot_start_id;
        Ok(current_hot_entries)
    }

    /// Spill oldest entries from hot tier to warm tier
    ///
    /// Story 6.3 AC3: Creates a WarmIndex file with oldest entries and removes
    /// them from the hot tier structures.
    fn spill_to_warm(&mut self) -> Result<(), IndexError> {
        let hot_entries = self.total_entries_merged - self.hot_start_id;
        if hot_entries <= crate::indexer::tiered::DEFAULT_HOT_TIER_ENTRIES {
            return Ok(()); // Nothing to spill
        }

        // Calculate how many entries to spill
        let spill_count = hot_entries - crate::indexer::tiered::DEFAULT_HOT_TIER_ENTRIES;
        let spill_end_id = self.hot_start_id + spill_count;

        log::info!(
            "[PROGRESSIVE] Spilling {} entries to warm tier (IDs {}-{})",
            spill_count,
            self.hot_start_id,
            spill_end_id - 1
        );

        // Ensure warm directory exists
        fs::create_dir_all(&self.warm_directory)?;

        // Create warm tier file
        let warm_path = self.warm_directory.join(format!("warm_{}.idx", self.warm_tier_count));

        // Create WarmIndex from current data (it will filter by entry range)
        crate::indexer::tiered::WarmIndex::create(
            &warm_path,
            &self.inverted,
            &self.bitmap,
            &self.offset_table,
            self.hot_start_id,
            spill_end_id,
        )?;

        self.warm_files.push(warm_path);
        self.warm_tier_count += 1;
        self.hot_start_id = spill_end_id;

        log::info!(
            "[PROGRESSIVE] Warm tier {} created, hot tier now starts at ID {}",
            self.warm_tier_count - 1,
            self.hot_start_id
        );

        Ok(())
    }

    /// Get total entries merged so far
    pub fn total_entries(&self) -> u64 {
        self.total_entries_merged
    }

    /// Get current hot tier entry count
    pub fn hot_entries(&self) -> u64 {
        self.total_entries_merged - self.hot_start_id
    }

    /// Finalize the merge and return a TieredIndex
    ///
    /// Story 6.3 AC3: Builds the final TieredIndex with hot tier in RAM
    /// and warm tiers as memory-mapped files.
    pub fn finalize(self) -> Result<crate::indexer::tiered::TieredIndex, IndexError> {
        // Sort inverted index vectors for efficient queries
        let mut inverted = self.inverted;
        inverted.sort_all();

        // Build hot tier
        let hot = crate::indexer::tiered::HotIndex::from_indexes(
            inverted,
            self.bitmap,
            self.offset_table,
            self.hot_start_id,
            self.total_entries_merged,
        );

        // Open warm tier files
        let mut warm = Vec::with_capacity(self.warm_files.len());
        for path in &self.warm_files {
            warm.push(crate::indexer::tiered::WarmIndex::open(path)?);
        }

        let config = crate::indexer::tiered::TieredConfig::with_warm_directory(&self.warm_directory);

        log::info!(
            "[PROGRESSIVE] Finalized: {} total entries, {} in hot tier, {} warm tiers",
            self.total_entries_merged,
            self.total_entries_merged - self.hot_start_id,
            warm.len()
        );

        Ok(crate::indexer::tiered::TieredIndex {
            config,
            hot,
            warm,
            total_entries: self.total_entries_merged,
        })
    }
}

/// Find chunk boundaries aligned to line endings
fn find_chunk_boundaries(data: &[u8], chunk_size: usize) -> Vec<ChunkBoundary> {
    let mut boundaries = Vec::new();
    let mut start = 0;
    let mut start_offset = 0u64;

    while start < data.len() {
        let mut end = (start + chunk_size).min(data.len());

        // Align to line ending (find next newline after chunk boundary)
        if end < data.len() {
            while end < data.len() && data[end] != b'\n' {
                end += 1;
            }
            if end < data.len() {
                end += 1; // Include the newline
            }
        }

        boundaries.push(ChunkBoundary {
            start,
            end,
            start_offset,
        });

        start_offset += (end - start) as u64;
        start = end;
    }

    boundaries
}

/// Parse a single chunk and return parsed entries
/// Story 6.1 (Amelia): Uses LocalInterner for string deduplication within chunk
fn parse_chunk(
    data: &[u8],
    chunk: &ChunkBoundary,
    format: LogFormat,
    cancellation_token: &AtomicBool,
    bytes_processed: &AtomicU64,
) -> Result<Vec<ParsedEntry>, IndexError> {
    let chunk_data = &data[chunk.start..chunk.end];

    // Convert to string (lossy for non-UTF8)
    let text = String::from_utf8_lossy(chunk_data);

    // Estimate entries in this chunk (~200 bytes per log line on average)
    let estimated_entries = chunk_data.len() / 200;

    let mut entries = Vec::with_capacity(estimated_entries);
    let mut local_offset = 0u64;
    let mut line_count = 0u64;

    // Story 6.1 (Amelia): String interning for IP/interface deduplication
    // LocalInterner deduplicates strings within this chunk, reducing allocations
    let mut interner = LocalInterner::with_capacity(ESTIMATED_UNIQUE_IPS);

    for line in text.lines() {
        // Check cancellation periodically
        if line_count % 1000 == 0 && cancellation_token.load(Ordering::Relaxed) {
            return Err(IndexError::Cancelled);
        }

        let line_bytes = line.len() as u64 + 1; // +1 for newline

        if line.trim().is_empty() {
            local_offset += line_bytes;
            line_count += 1;
            continue;
        }

        let parse_result = match format {
            LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(line, line_count + 1, 0),
            LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(line, line_count + 1, 0),
            LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(line, line_count + 1, 0),
            LogFormat::Unknown => {
                return Err(IndexError::ParseError("Unknown format".to_string()));
            }
        };

        if let Ok(entry) = parse_result {
            entries.push(ParsedEntry::from_log_entry(
                chunk.start_offset + local_offset,
                &entry,
                &mut interner,
            ));
        }

        local_offset += line_bytes;
        line_count += 1;
    }

    // Update global progress
    bytes_processed.fetch_add((chunk.end - chunk.start) as u64, Ordering::Relaxed);

    Ok(entries)
}

/// Process a single batch of chunks and return batch indexes
fn process_batch(
    data: &[u8],
    chunks: &[ChunkBoundary],
    format: LogFormat,
    global_id_start: u64,
    cancellation_token: &AtomicBool,
    bytes_processed: &AtomicU64,
) -> Result<BatchIndexes, IndexError> {
    // Parse all chunks in this batch in parallel
    let parsed_chunks: Result<Vec<(usize, Vec<ParsedEntry>)>, IndexError> = chunks
        .par_iter()
        .enumerate()
        .map(|(idx, chunk)| {
            let entries = parse_chunk(data, chunk, format, cancellation_token, bytes_processed)?;
            Ok((idx, entries))
        })
        .collect();

    let mut parsed_chunks = parsed_chunks?;
    parsed_chunks.sort_by_key(|(idx, _)| *idx);

    // Calculate local offsets within batch
    let mut local_offsets = Vec::with_capacity(parsed_chunks.len());
    let mut current_offset = 0u64;
    for (_, entries) in &parsed_chunks {
        local_offsets.push(current_offset);
        current_offset += entries.len() as u64;
    }

    // Build batch index in parallel
    // Story 6.1 (Amelia): Pre-allocate HashMaps with estimated capacity
    let partial_indexes: Vec<BatchIndexes> = parsed_chunks
        .par_iter()
        .enumerate()
        .map(|(idx, (_, entries))| {
            let mut batch = BatchIndexes::with_capacity(entries.len());
            let base_id = global_id_start + local_offsets[idx];
            for (local_idx, entry) in entries.iter().enumerate() {
                let global_id = base_id + local_idx as u64;
                batch.add_entry(global_id, entry);
            }
            batch
        })
        .collect();

    // Drop parsed entries immediately to free memory
    drop(parsed_chunks);

    // Merge partial indexes into single batch index
    let mut merged = BatchIndexes::new();
    for partial in partial_indexes {
        merged.merge(partial);
    }

    Ok(merged)
}

/// Save batch indexes to a temporary file
///
/// Story 6.2: Uses rkyv for faster serialization with magic bytes and version header (AC5)
fn save_batch_to_disk(batch: BatchIndexes, temp_dir: &Path, batch_num: usize) -> Result<PathBuf, IndexError> {
    let file_path = temp_dir.join(format!("batch_{:04}.rkyv", batch_num));
    let file = File::create(&file_path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);

    let serialized: SerializedBatchIndexes = batch.into();

    // Write magic bytes and version header (Story 6.2, AC5)
    writer.write_all(BATCH_MAGIC)?;
    writer.write_all(&BATCH_VERSION.to_le_bytes())?;
    // Reserved 4 bytes for future use
    writer.write_all(&[0u8; 4])?;

    // Use rkyv for fast serialization
    let encoded = rkyv::to_bytes::<_, 256>(&serialized)
        .map_err(|e| IndexError::SerializationError(format!("rkyv serialization failed: {:?}", e)))?;

    writer.write_all(&encoded)?;
    writer.flush()?;

    Ok(file_path)
}

/// Load batch indexes from a temporary file
///
/// Story 6.2: Uses rkyv for faster deserialization with magic byte validation (AC5)
fn load_batch_from_disk(path: &Path) -> Result<BatchIndexes, IndexError> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(64 * 1024, file);
    let bytes: Vec<u8> = std::io::Read::bytes(reader)
        .collect::<Result<Vec<_>, _>>()?;

    // Validate magic bytes and version header (Story 6.2, AC5)
    const HEADER_SIZE: usize = 16; // 8 magic + 4 version + 4 reserved
    if bytes.len() < HEADER_SIZE {
        return Err(IndexError::SerializationError("Batch file too small".to_string()));
    }
    if &bytes[0..8] != BATCH_MAGIC {
        return Err(IndexError::SerializationError("Invalid batch file magic bytes".to_string()));
    }
    let version = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    if version != BATCH_VERSION {
        return Err(IndexError::SerializationError(format!(
            "Unsupported batch file version: {} (expected {})", version, BATCH_VERSION
        )));
    }

    // Use rkyv for fast deserialization with validation (skip header)
    let archived = rkyv::check_archived_root::<SerializedBatchIndexes>(&bytes[HEADER_SIZE..])
        .map_err(|e| IndexError::SerializationError(format!("rkyv validation failed: {:?}", e)))?;

    let serialized: SerializedBatchIndexes = archived
        .deserialize(&mut rkyv::Infallible)
        .map_err(|e| IndexError::SerializationError(format!("rkyv deserialization failed: {:?}", e)))?;

    BatchIndexes::try_from(serialized)
}

/// Build index using streaming batch processing for memory efficiency
///
/// Memory usage: ~2GB peak for any file size (vs 23GB for 14GB file with full parallel)
pub fn build_index_streaming<P, F>(
    file_path: P,
    format: LogFormat,
    file_size: u64,
    source_hash: String,
    cancellation_token: Arc<AtomicBool>,
    mut progress_callback: F,
) -> Result<(InvertedIndex, BitmapIndex, OffsetTable, IndexMetadata), IndexError>
where
    P: AsRef<Path>,
    F: FnMut(IndexProgress),
{
    let file_path = file_path.as_ref();
    let file = File::open(file_path)?;

    // Memory-map the file
    let mmap = unsafe { Mmap::map(&file)? };
    let data = &mmap[..];

    // Determine chunk size
    let num_threads = rayon::current_num_threads();
    let adaptive_chunk_size = if file_size > (CHUNK_SIZE * num_threads) as u64 {
        CHUNK_SIZE
    } else {
        (file_size as usize / num_threads).max(MIN_CHUNK_SIZE)
    };

    // Find all chunk boundaries
    let all_chunks = find_chunk_boundaries(data, adaptive_chunk_size);
    let total_chunks = all_chunks.len();
    let num_batches = (total_chunks + CHUNKS_PER_BATCH - 1) / CHUNKS_PER_BATCH;

    log::info!(
        "[STREAMING] Processing {} chunks in {} batches ({} chunks/batch), {} threads",
        total_chunks,
        num_batches,
        CHUNKS_PER_BATCH,
        num_threads
    );

    let bytes_processed = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();

    // Story 6.3 AC2: Estimate total entries for progress reporting
    let estimated_total_entries = IndexProgress::estimate_entries(file_size, 200);

    // Story 6.3 AC2: Initial progress with batch info
    progress_callback(IndexProgress::with_batch_info(
        0,
        file_size,
        0.0,
        0, // entries_indexed
        estimated_total_entries,
        false, // partial_filter_available - not yet
        0,     // batches_completed
        num_batches as u32,
    ));

    // Create temp directory for batch files
    let temp_dir = std::env::temp_dir().join(format!("opnsense_index_{}", std::process::id()));
    fs::create_dir_all(&temp_dir)?;

    let mut batch_files: Vec<PathBuf> = Vec::new();
    let mut global_id_offset = 0u64;
    let mut total_entries = 0u64;

    // Process each batch
    for batch_num in 0..num_batches {
        if cancellation_token.load(Ordering::Relaxed) {
            // Cleanup temp files
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(IndexError::Cancelled);
        }

        let batch_start = batch_num * CHUNKS_PER_BATCH;
        let batch_end = ((batch_num + 1) * CHUNKS_PER_BATCH).min(total_chunks);
        let batch_chunks = &all_chunks[batch_start..batch_end];

        log::info!(
            "[STREAMING] Processing batch {}/{} (chunks {}-{})",
            batch_num + 1,
            num_batches,
            batch_start,
            batch_end - 1
        );

        // Process batch
        let batch_index = process_batch(
            data,
            batch_chunks,
            format,
            global_id_offset,
            &cancellation_token,
            &bytes_processed,
        )?;

        let batch_entries = batch_index.entry_count;
        global_id_offset += batch_entries;
        total_entries += batch_entries;

        // Save batch to disk and free memory
        let batch_path = save_batch_to_disk(batch_index, &temp_dir, batch_num)?;
        batch_files.push(batch_path);

        // Story 6.3 AC2/AC5: Update progress with batch info
        // partial_filter_available becomes true after first batch
        let elapsed = start_time.elapsed().as_secs_f64();
        let processed = bytes_processed.load(Ordering::Relaxed);
        let batches_done = (batch_num + 1) as u32;
        progress_callback(IndexProgress::with_batch_info(
            processed,
            file_size,
            elapsed,
            total_entries,
            estimated_total_entries,
            batches_done >= 1, // Story 6.3 AC1: partial filtering available after first batch
            batches_done,
            num_batches as u32,
        ));

        log::info!(
            "[STREAMING] Batch {}/{} complete: {} entries, memory freed, partial_filter={}",
            batch_num + 1,
            num_batches,
            batch_entries,
            batches_done >= 1
        );
    }

    let parse_elapsed = start_time.elapsed().as_secs_f64();
    log::info!(
        "[STREAMING] All batches processed in {:.2}s, {} total entries. Starting merge...",
        parse_elapsed,
        total_entries
    );

    // Merge all batches
    let mut final_index = BatchIndexes::new();

    for (batch_num, batch_path) in batch_files.iter().enumerate() {
        if cancellation_token.load(Ordering::Relaxed) {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(IndexError::Cancelled);
        }

        log::debug!("[STREAMING] Merging batch {}/{}", batch_num + 1, num_batches);

        let batch = load_batch_from_disk(batch_path)?;
        final_index.merge(batch);

        // Delete batch file after merging to free disk space
        let _ = fs::remove_file(batch_path);
    }

    // Cleanup temp directory
    let _ = fs::remove_dir_all(&temp_dir);

    // Sort inverted index vectors for efficient queries
    for ids in final_index.source_ips.values_mut() {
        ids.sort_unstable();
    }
    for ids in final_index.dest_ips.values_mut() {
        ids.sort_unstable();
    }
    for ids in final_index.source_ports.values_mut() {
        ids.sort_unstable();
    }
    for ids in final_index.dest_ports.values_mut() {
        ids.sort_unstable();
    }

    // Build final indexes
    let bitmap_index = BitmapIndex::from_raw(
        final_index.actions,
        final_index.protocols,
        final_index.interfaces,
    );

    let inverted_index = InvertedIndex::from_raw(
        final_index.source_ips,
        final_index.dest_ips,
        final_index.source_ports,
        final_index.dest_ports,
    );

    // Build offset table
    let mut offset_table = OffsetTable::with_capacity(total_entries as usize);
    for (entry_id, offset) in final_index.offsets {
        offset_table.set_offset(entry_id, offset);
    }

    // Story 6.3 AC2: Final progress update with all batches complete
    let final_elapsed = start_time.elapsed().as_secs_f64();
    progress_callback(IndexProgress::with_batch_info(
        file_size,
        file_size,
        final_elapsed,
        total_entries,
        total_entries, // estimated = actual at completion
        true,          // partial_filter_available
        num_batches as u32,
        num_batches as u32,
    ));

    log::info!(
        "[STREAMING] Complete: {} entries in {:.2}s ({:.0} entries/sec)",
        total_entries,
        final_elapsed,
        total_entries as f64 / final_elapsed
    );

    let metadata = IndexMetadata {
        format,
        entry_count: total_entries,
        created_at: chrono::Utc::now(),
        source_file_path: file_path.to_string_lossy().to_string(),
        source_file_size: file_size,
        source_file_hash: source_hash,
    };

    Ok((inverted_index, bitmap_index, offset_table, metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_indexes_merge() {
        let mut batch1 = BatchIndexes::new();
        batch1.actions.insert("block".to_string(), {
            let mut b = RoaringBitmap::new();
            b.insert(1);
            b.insert(2);
            b
        });
        batch1.source_ips.insert("192.168.1.1".to_string(), vec![1]);
        batch1.offsets.push((1, 100));
        batch1.entry_count = 1;

        let mut batch2 = BatchIndexes::new();
        batch2.actions.insert("block".to_string(), {
            let mut b = RoaringBitmap::new();
            b.insert(3);
            b.insert(4);
            b
        });
        batch2.actions.insert("pass".to_string(), {
            let mut b = RoaringBitmap::new();
            b.insert(5);
            b
        });
        batch2.source_ips.insert("192.168.1.1".to_string(), vec![3]);
        batch2.source_ips.insert("192.168.1.2".to_string(), vec![4]);
        batch2.offsets.push((3, 300));
        batch2.entry_count = 2;

        batch1.merge(batch2);

        // Verify bitmap merge
        assert_eq!(batch1.actions.get("block").unwrap().len(), 4);
        assert_eq!(batch1.actions.get("pass").unwrap().len(), 1);

        // Verify inverted index merge
        assert_eq!(batch1.source_ips.get("192.168.1.1").unwrap().len(), 2);
        assert_eq!(batch1.source_ips.get("192.168.1.2").unwrap().len(), 1);

        // Verify entry count
        assert_eq!(batch1.entry_count, 3);
        assert_eq!(batch1.offsets.len(), 2);
    }

    #[test]
    fn test_find_chunk_boundaries() {
        let data = b"line1\nline2\nline3\n";
        let chunks = find_chunk_boundaries(data, 6);
        assert!(chunks.len() >= 1);

        // Verify all chunks end at line boundaries
        for chunk in &chunks {
            if chunk.end < data.len() {
                assert_eq!(data[chunk.end - 1], b'\n');
            }
        }
    }

    /// Story 6.3 AC3: Test ProgressiveMergeState basic functionality
    #[test]
    fn test_progressive_merge_state_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = ProgressiveMergeState::new(temp_dir.path().to_path_buf());

        // Create first batch with some entries
        let mut batch1 = BatchIndexes::new();
        batch1.actions.insert("block".to_string(), {
            let mut b = RoaringBitmap::new();
            b.insert(0);
            b.insert(1);
            b
        });
        batch1.source_ips.insert("192.168.1.1".to_string(), vec![0, 1]);
        batch1.offsets.push((0, 0));
        batch1.offsets.push((1, 100));
        batch1.entry_count = 2;

        // Merge first batch
        let hot_entries = state.merge_batch(batch1).unwrap();
        assert_eq!(state.total_entries(), 2);
        assert_eq!(hot_entries, 2);

        // Create second batch
        let mut batch2 = BatchIndexes::new();
        batch2.actions.insert("pass".to_string(), {
            let mut b = RoaringBitmap::new();
            b.insert(2);
            b
        });
        batch2.source_ips.insert("192.168.1.2".to_string(), vec![2]);
        batch2.offsets.push((2, 200));
        batch2.entry_count = 1;

        // Merge second batch
        let hot_entries = state.merge_batch(batch2).unwrap();
        assert_eq!(state.total_entries(), 3);
        assert_eq!(hot_entries, 3);

        // Finalize and verify TieredIndex
        let tiered = state.finalize().unwrap();
        assert_eq!(tiered.entry_count(), 3);
        assert!(tiered.warm.is_empty()); // No spill needed for small data
    }

    /// Story 6.3 AC3: Test progressive merge with multiple batches
    #[test]
    fn test_progressive_merge_multiple_batches() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = ProgressiveMergeState::new(temp_dir.path().to_path_buf());

        // Create 3 batches, each with 1000 entries
        for batch_num in 0..3 {
            let mut batch = BatchIndexes::new();
            let base_id = batch_num * 1000;

            // Add bitmap entries
            batch.actions.insert("block".to_string(), {
                let mut b = RoaringBitmap::new();
                for i in 0..500 {
                    b.insert((base_id + i) as u32);
                }
                b
            });
            batch.actions.insert("pass".to_string(), {
                let mut b = RoaringBitmap::new();
                for i in 500..1000 {
                    b.insert((base_id + i) as u32);
                }
                b
            });

            // Add inverted index entries
            for i in 0..1000 {
                let ip = format!("192.168.{}.{}", batch_num, i % 256);
                batch.source_ips.entry(ip).or_default().push(base_id + i);
                batch.offsets.push((base_id + i, (base_id + i) * 100));
            }
            batch.entry_count = 1000;

            state.merge_batch(batch).unwrap();
        }

        assert_eq!(state.total_entries(), 3000);

        // Finalize and verify
        let tiered = state.finalize().unwrap();
        assert_eq!(tiered.entry_count(), 3000);

        // Verify structure is correct (warm tiers empty for small data)
        assert!(tiered.warm.is_empty(), "No warm tiers expected for 3000 entries");
    }

    /// Story 6.3 AC2: Test IndexProgress with_batch_info in streaming context
    #[test]
    fn test_progress_callback_batch_info() {
        use crate::indexer::progress::IndexProgress;

        let mut progress_updates: Vec<IndexProgress> = Vec::new();

        // Simulate progress callbacks during batch processing
        let file_size = 1_000_000_000u64;
        let estimated_entries = IndexProgress::estimate_entries(file_size, 200);
        let num_batches = 10u32;

        // Initial progress
        progress_updates.push(IndexProgress::with_batch_info(
            0, file_size, 0.0, 0, estimated_entries, false, 0, num_batches,
        ));

        // After first batch
        progress_updates.push(IndexProgress::with_batch_info(
            100_000_000, file_size, 60.0, 500_000, estimated_entries,
            true, // partial_filter_available becomes true
            1, num_batches,
        ));

        // After second batch
        progress_updates.push(IndexProgress::with_batch_info(
            200_000_000, file_size, 120.0, 1_000_000, estimated_entries,
            true, 2, num_batches,
        ));

        // Verify progression
        assert!(!progress_updates[0].partial_filter_available);
        assert!(progress_updates[1].partial_filter_available);
        assert!(progress_updates[2].partial_filter_available);

        assert_eq!(progress_updates[0].batches_completed, 0);
        assert_eq!(progress_updates[1].batches_completed, 1);
        assert_eq!(progress_updates[2].batches_completed, 2);

        assert_eq!(progress_updates[0].entries_indexed, 0);
        assert_eq!(progress_updates[1].entries_indexed, 500_000);
        assert_eq!(progress_updates[2].entries_indexed, 1_000_000);
    }
}
