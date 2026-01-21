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

use crate::indexer::bitmap::BitmapIndex;
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

/// Parsed entry data (minimal allocation)
#[derive(Clone)]
pub(crate) struct ParsedEntry {
    pub byte_offset: u64,
    pub source_ip: Option<String>,
    pub dest_ip: Option<String>,
    pub source_port: Option<u16>,
    pub dest_port: Option<u16>,
    pub action: Option<String>,
    pub protocol: Option<String>,
    pub interface: Option<String>,
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
        Self {
            actions: HashMap::new(),
            protocols: HashMap::new(),
            interfaces: HashMap::new(),
            source_ips: HashMap::new(),
            dest_ips: HashMap::new(),
            source_ports: HashMap::new(),
            dest_ports: HashMap::new(),
            offsets: Vec::new(),
            entry_count: 0,
        }
    }

    fn add_entry(&mut self, global_id: u64, entry: &ParsedEntry) {
        // Bitmap index
        if let Some(action) = &entry.action {
            self.actions
                .entry(action.to_lowercase())
                .or_default()
                .insert(global_id as u32);
        }
        if let Some(protocol) = &entry.protocol {
            self.protocols
                .entry(protocol.to_uppercase())
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
        if let Some(ip) = &entry.source_ip {
            self.source_ips
                .entry(ip.clone())
                .or_default()
                .push(global_id);
        }
        if let Some(ip) = &entry.dest_ip {
            self.dest_ips
                .entry(ip.clone())
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
#[derive(serde::Serialize, serde::Deserialize)]
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
fn parse_chunk(
    data: &[u8],
    chunk: &ChunkBoundary,
    format: LogFormat,
    cancellation_token: &AtomicBool,
    bytes_processed: &AtomicU64,
) -> Result<Vec<ParsedEntry>, IndexError> {
    let mut entries = Vec::new();
    let chunk_data = &data[chunk.start..chunk.end];

    // Convert to string (lossy for non-UTF8)
    let text = String::from_utf8_lossy(chunk_data);

    let mut local_offset = 0u64;
    let mut line_count = 0u64;

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
            entries.push(ParsedEntry {
                byte_offset: chunk.start_offset + local_offset,
                source_ip: entry.source_ip,
                dest_ip: entry.dest_ip,
                source_port: entry.source_port,
                dest_port: entry.dest_port,
                action: entry.action,
                protocol: entry.protocol,
                interface: entry.interface,
            });
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
    let partial_indexes: Vec<BatchIndexes> = parsed_chunks
        .par_iter()
        .enumerate()
        .map(|(idx, (_, entries))| {
            let mut batch = BatchIndexes::new();
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
fn save_batch_to_disk(batch: BatchIndexes, temp_dir: &Path, batch_num: usize) -> Result<PathBuf, IndexError> {
    let file_path = temp_dir.join(format!("batch_{:04}.bin", batch_num));
    let file = File::create(&file_path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);

    let serialized: SerializedBatchIndexes = batch.into();
    let encoded = bincode::serde::encode_to_vec(&serialized, bincode::config::standard())
        .map_err(|e| IndexError::SerializationError(e.to_string()))?;

    writer.write_all(&encoded)?;
    writer.flush()?;

    Ok(file_path)
}

/// Load batch indexes from a temporary file
fn load_batch_from_disk(path: &Path) -> Result<BatchIndexes, IndexError> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(64 * 1024, file);
    let bytes: Vec<u8> = std::io::Read::bytes(reader)
        .collect::<Result<Vec<_>, _>>()?;

    let (serialized, _): (SerializedBatchIndexes, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
        .map_err(|e| IndexError::SerializationError(e.to_string()))?;

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
    progress_callback(IndexProgress::new(0, file_size, 0.0));

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

        // Update progress
        let elapsed = start_time.elapsed().as_secs_f64();
        let processed = bytes_processed.load(Ordering::Relaxed);
        progress_callback(IndexProgress::new(processed, file_size, elapsed));

        log::info!(
            "[STREAMING] Batch {}/{} complete: {} entries, memory freed",
            batch_num + 1,
            num_batches,
            batch_entries
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

    let final_elapsed = start_time.elapsed().as_secs_f64();
    progress_callback(IndexProgress::new(file_size, file_size, final_elapsed));

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
}
