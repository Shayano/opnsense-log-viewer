//! Parallel indexing module for high-performance log file processing
//!
//! This module uses memory-mapped files and rayon for parallel parsing,
//! achieving significant speedups on multi-core systems for large log files.

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use memmap2::Mmap;
use rayon::prelude::*;

use crate::indexer::bitmap::BitmapIndex;
use crate::indexer::inverted::InvertedIndex;
use crate::indexer::offset_table::OffsetTable;
use crate::indexer::progress::IndexProgress;
use crate::indexer::hybrid::{IndexError, IndexMetadata};
use crate::parser::{csv_filterlog, rfc3164, rfc5424};
use crate::types::log_entry::LogFormat;

/// Target chunk size in bytes (10 MB)
/// This provides good parallelism while keeping memory overhead reasonable
const CHUNK_SIZE: usize = 10 * 1024 * 1024;

/// Minimum chunk size to avoid overhead on small files
const MIN_CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

/// Partial index built from a single chunk
/// These are merged after parallel processing
struct PartialIndex {
    /// Entries as (local_entry_id, byte_offset, line_data)
    entries: Vec<(u64, u64, ParsedEntry)>,
}

/// Parsed entry data (minimal allocation)
struct ParsedEntry {
    source_ip: Option<String>,
    dest_ip: Option<String>,
    source_port: Option<u16>,
    dest_port: Option<u16>,
    action: Option<String>,
    protocol: Option<String>,
    interface: Option<String>,
}

/// Chunk boundaries for parallel processing
struct ChunkBoundary {
    start: usize,
    end: usize,
    start_offset: u64,
}

impl PartialIndex {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
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

/// Parse a single chunk and build partial index
fn parse_chunk(
    data: &[u8],
    chunk: &ChunkBoundary,
    format: LogFormat,
    base_entry_id: u64,
    cancellation_token: &AtomicBool,
    bytes_processed: &AtomicU64,
) -> Result<PartialIndex, IndexError> {
    let mut partial = PartialIndex::new();
    let chunk_data = &data[chunk.start..chunk.end];

    // Convert to string (lossy for non-UTF8)
    let text = String::from_utf8_lossy(chunk_data);

    let mut local_entry_id = 0u64;
    let mut local_offset = 0u64;

    for line in text.lines() {
        // Check cancellation periodically
        if local_entry_id % 1000 == 0 && cancellation_token.load(Ordering::Relaxed) {
            return Err(IndexError::Cancelled);
        }

        let line_bytes = line.len() as u64 + 1; // +1 for newline

        if line.trim().is_empty() {
            local_offset += line_bytes;
            continue;
        }

        let parse_result = match format {
            LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(line, local_entry_id + 1, base_entry_id + local_entry_id),
            LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(line, local_entry_id + 1, base_entry_id + local_entry_id),
            LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(line, local_entry_id + 1, base_entry_id + local_entry_id),
            LogFormat::Unknown => {
                return Err(IndexError::ParseError("Unknown format".to_string()));
            }
        };

        if let Ok(entry) = parse_result {
            let parsed = ParsedEntry {
                source_ip: entry.source_ip.clone(),
                dest_ip: entry.dest_ip.clone(),
                source_port: entry.source_port,
                dest_port: entry.dest_port,
                action: entry.action.clone(),
                protocol: entry.protocol.clone(),
                interface: entry.interface.clone(),
            };

            partial.entries.push((
                local_entry_id,
                chunk.start_offset + local_offset,
                parsed,
            ));

            local_entry_id += 1;
        }

        local_offset += line_bytes;
    }

    // Update global progress
    bytes_processed.fetch_add((chunk.end - chunk.start) as u64, Ordering::Relaxed);

    Ok(partial)
}

/// Merge partial indexes into final indexes
fn merge_partial_indexes(
    partials: Vec<(usize, PartialIndex)>,
    inverted: &mut InvertedIndex,
    bitmap: &mut BitmapIndex,
    offset_table: &mut OffsetTable,
) -> u64 {
    // Sort by chunk index to maintain order
    let mut sorted_partials: Vec<_> = partials;
    sorted_partials.sort_by_key(|(idx, _)| *idx);

    let mut global_entry_id = 0u64;

    for (_, partial) in sorted_partials {
        for (_local_id, byte_offset, entry) in partial.entries {
            // Add to inverted index
            inverted.add_entry(
                global_entry_id,
                entry.source_ip.as_deref(),
                entry.dest_ip.as_deref(),
                entry.source_port,
                entry.dest_port,
            );

            // Add to bitmap index
            bitmap.add_entry(
                global_entry_id,
                entry.action.as_deref(),
                entry.protocol.as_deref(),
                entry.interface.as_deref(),
            );

            // Add to offset table
            offset_table.add_offset(global_entry_id, byte_offset);

            global_entry_id += 1;
        }
    }

    global_entry_id
}

/// Build index using parallel processing
///
/// This function memory-maps the file, splits it into chunks at line boundaries,
/// parses each chunk in parallel using rayon, and merges the results.
///
/// # Performance
/// - Uses all available CPU cores via rayon
/// - Memory-mapped I/O avoids copying data
/// - 10 MB chunks balance parallelism vs overhead
pub fn build_index_parallel<P, F>(
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

    // Determine chunk size based on file size and available parallelism
    let num_threads = rayon::current_num_threads();
    let adaptive_chunk_size = if file_size > (CHUNK_SIZE * num_threads) as u64 {
        CHUNK_SIZE
    } else {
        (file_size as usize / num_threads).max(MIN_CHUNK_SIZE)
    };

    // Find chunk boundaries aligned to line endings
    let chunks = find_chunk_boundaries(data, adaptive_chunk_size);
    let total_chunks = chunks.len();

    log::info!(
        "[PARALLEL] Starting parallel indexation: {} chunks, {} threads, chunk_size={}MB",
        total_chunks,
        num_threads,
        adaptive_chunk_size / (1024 * 1024)
    );

    // Shared progress tracking
    let bytes_processed = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();

    // Initial progress
    progress_callback(IndexProgress::new(0, file_size, 0.0));

    // Parse chunks in parallel
    let results: Result<Vec<(usize, PartialIndex)>, IndexError> = chunks
        .par_iter()
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            // Calculate base entry ID (approximate, will be fixed during merge)
            let base_entry_id = 0; // We'll reassign during merge

            let partial = parse_chunk(
                data,
                chunk,
                format,
                base_entry_id,
                &cancellation_token,
                &bytes_processed,
            )?;

            Ok((chunk_idx, partial))
        })
        .collect();

    // Check for errors
    let partials = results?;

    // Emit progress after parallel phase
    let elapsed = start_time.elapsed().as_secs_f64();
    progress_callback(IndexProgress::new(file_size / 2, file_size, elapsed));

    // Merge partial indexes (sequential but fast)
    let mut inverted = InvertedIndex::new();
    let mut bitmap = BitmapIndex::new();
    let mut offset_table = OffsetTable::new();

    let total_entries = merge_partial_indexes(partials, &mut inverted, &mut bitmap, &mut offset_table);

    // Final progress
    let final_elapsed = start_time.elapsed().as_secs_f64();
    progress_callback(IndexProgress::new(file_size, file_size, final_elapsed));

    log::info!(
        "[PARALLEL] Indexation complete: {} entries in {:.2}s ({:.0} entries/sec)",
        total_entries,
        final_elapsed,
        total_entries as f64 / final_elapsed
    );

    // Build metadata
    let metadata = IndexMetadata {
        format,
        entry_count: total_entries,
        created_at: chrono::Utc::now(),
        source_file_path: file_path.to_string_lossy().to_string(),
        source_file_size: file_size,
        source_file_hash: source_hash,
    };

    Ok((inverted, bitmap, offset_table, metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_chunk_boundaries_simple() {
        let data = b"line1\nline2\nline3\n";
        let chunks = find_chunk_boundaries(data, 6);

        // Should split at line boundaries
        assert!(chunks.len() >= 1);

        // First chunk should start at 0
        assert_eq!(chunks[0].start, 0);
    }

    #[test]
    fn test_find_chunk_boundaries_large_chunk() {
        let data = b"line1\nline2\nline3\n";
        let chunks = find_chunk_boundaries(data, 1000);

        // Single chunk for small data
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, data.len());
    }

    #[test]
    fn test_chunk_boundary_alignment() {
        // Create data with known line boundaries
        let mut data = Vec::new();
        for i in 0..100 {
            data.extend_from_slice(format!("line{}\n", i).as_bytes());
        }

        let chunks = find_chunk_boundaries(&data, 50);

        // Each chunk should end at a newline
        for chunk in &chunks {
            if chunk.end < data.len() {
                assert_eq!(data[chunk.end - 1], b'\n');
            }
        }
    }
}
