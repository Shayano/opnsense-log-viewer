//! Parallel indexing module for high-performance log file processing
//!
//! This module uses memory-mapped files and rayon for parallel parsing AND merging,
//! achieving significant speedups on multi-core systems for large log files.
//!
//! Architecture:
//! 1. Phase 1 (parallel): Parse chunks into ParsedEntry vectors
//! 2. Phase 2 (fast sequential): Calculate entry ID offsets per chunk
//! 3. Phase 3 (parallel): Build partial indexes per chunk with correct global IDs
//! 4. Phase 4 (parallel): Merge partial indexes into final indexes

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
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

/// Parsed entry data (minimal allocation)
#[derive(Clone)]
struct ParsedEntry {
    byte_offset: u64,
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

/// Partial indexes built from a chunk (for parallel merge)
struct PartialIndexes {
    // Bitmap index components
    actions: HashMap<String, RoaringBitmap>,
    protocols: HashMap<String, RoaringBitmap>,
    interfaces: HashMap<String, RoaringBitmap>,
    // Inverted index components
    source_ips: HashMap<String, Vec<u64>>,
    dest_ips: HashMap<String, Vec<u64>>,
    source_ports: HashMap<u16, Vec<u64>>,
    dest_ports: HashMap<u16, Vec<u64>>,
    // Offset data: (global_entry_id, byte_offset)
    offsets: Vec<(u64, u64)>,
}

impl PartialIndexes {
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

/// Build partial indexes from entries with assigned global IDs (parallel)
fn build_partial_indexes(
    entries: &[ParsedEntry],
    global_id_offset: u64,
) -> PartialIndexes {
    let mut partial = PartialIndexes::new();

    for (local_idx, entry) in entries.iter().enumerate() {
        let global_id = global_id_offset + local_idx as u64;
        partial.add_entry(global_id, entry);
    }

    partial
}

/// Merge bitmap HashMaps in parallel
fn merge_bitmap_maps(
    maps: Vec<HashMap<String, RoaringBitmap>>,
) -> HashMap<String, RoaringBitmap> {
    if maps.is_empty() {
        return HashMap::new();
    }
    if maps.len() == 1 {
        return maps.into_iter().next().unwrap();
    }

    // Collect all unique keys
    let all_keys: std::collections::HashSet<String> = maps
        .iter()
        .flat_map(|m| m.keys().cloned())
        .collect();

    // Merge in parallel by key
    all_keys
        .into_par_iter()
        .map(|key| {
            let mut merged = RoaringBitmap::new();
            for map in &maps {
                if let Some(bitmap) = map.get(&key) {
                    merged |= bitmap;
                }
            }
            (key, merged)
        })
        .collect()
}

/// Merge inverted index string maps
fn merge_string_maps(
    maps: Vec<HashMap<String, Vec<u64>>>,
) -> HashMap<String, Vec<u64>> {
    if maps.is_empty() {
        return HashMap::new();
    }
    if maps.len() == 1 {
        return maps.into_iter().next().unwrap();
    }

    let all_keys: std::collections::HashSet<String> = maps
        .iter()
        .flat_map(|m| m.keys().cloned())
        .collect();

    all_keys
        .into_par_iter()
        .map(|key| {
            let mut merged = Vec::new();
            for map in &maps {
                if let Some(ids) = map.get(&key) {
                    merged.extend(ids.iter().copied());
                }
            }
            merged.sort_unstable();
            (key, merged)
        })
        .collect()
}

/// Merge inverted index u16 maps (for ports)
fn merge_port_maps(
    maps: Vec<HashMap<u16, Vec<u64>>>,
) -> HashMap<u16, Vec<u64>> {
    if maps.is_empty() {
        return HashMap::new();
    }
    if maps.len() == 1 {
        return maps.into_iter().next().unwrap();
    }

    let all_keys: std::collections::HashSet<u16> = maps
        .iter()
        .flat_map(|m| m.keys().copied())
        .collect();

    all_keys
        .into_par_iter()
        .map(|key| {
            let mut merged = Vec::new();
            for map in &maps {
                if let Some(ids) = map.get(&key) {
                    merged.extend(ids.iter().copied());
                }
            }
            merged.sort_unstable();
            (key, merged)
        })
        .collect()
}

/// Build index using fully parallel processing
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
        "[PARALLEL] Phase 1: Parsing {} chunks with {} threads, chunk_size={}MB",
        total_chunks,
        num_threads,
        adaptive_chunk_size / (1024 * 1024)
    );

    let bytes_processed = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();

    progress_callback(IndexProgress::new(0, file_size, 0.0));

    // ========== PHASE 1: Parse chunks in parallel ==========
    let parsed_chunks: Result<Vec<(usize, Vec<ParsedEntry>)>, IndexError> = chunks
        .par_iter()
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            let entries = parse_chunk(
                data,
                chunk,
                format,
                &cancellation_token,
                &bytes_processed,
            )?;
            Ok((chunk_idx, entries))
        })
        .collect();

    let mut parsed_chunks = parsed_chunks?;
    parsed_chunks.sort_by_key(|(idx, _)| *idx);

    let phase1_elapsed = start_time.elapsed().as_secs_f64();
    log::info!("[PARALLEL] Phase 1 complete: parsing done in {:.2}s", phase1_elapsed);
    progress_callback(IndexProgress::new(file_size / 3, file_size, phase1_elapsed));

    // ========== PHASE 2: Calculate global ID offsets (sequential but O(n_chunks)) ==========
    let mut global_id_offsets = Vec::with_capacity(parsed_chunks.len());
    let mut current_offset = 0u64;
    for (_, entries) in &parsed_chunks {
        global_id_offsets.push(current_offset);
        current_offset += entries.len() as u64;
    }
    let total_entries = current_offset;

    log::info!(
        "[PARALLEL] Phase 2: {} total entries across {} chunks",
        total_entries,
        parsed_chunks.len()
    );

    // ========== PHASE 3: Build partial indexes in parallel ==========
    log::info!("[PARALLEL] Phase 3: Building partial indexes in parallel");

    let partial_indexes: Vec<PartialIndexes> = parsed_chunks
        .par_iter()
        .enumerate()
        .map(|(idx, (_, entries))| {
            build_partial_indexes(entries, global_id_offsets[idx])
        })
        .collect();

    // Drop parsed entries to free memory
    drop(parsed_chunks);

    let phase3_elapsed = start_time.elapsed().as_secs_f64();
    log::info!("[PARALLEL] Phase 3 complete: partial indexes built in {:.2}s", phase3_elapsed - phase1_elapsed);
    progress_callback(IndexProgress::new(file_size * 2 / 3, file_size, phase3_elapsed));

    // ========== PHASE 4: Merge partial indexes in parallel ==========
    log::info!("[PARALLEL] Phase 4: Merging indexes in parallel");

    // Extract components for parallel merge
    let (actions_maps, protocols_maps, interfaces_maps): (Vec<_>, Vec<_>, Vec<_>) = partial_indexes
        .iter()
        .map(|p| (p.actions.clone(), p.protocols.clone(), p.interfaces.clone()))
        .fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut a, mut p, mut i), (am, pm, im)| {
                a.push(am);
                p.push(pm);
                i.push(im);
                (a, p, i)
            },
        );

    let (src_ip_maps, dst_ip_maps, src_port_maps, dst_port_maps): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) = partial_indexes
        .iter()
        .map(|p| (
            p.source_ips.clone(),
            p.dest_ips.clone(),
            p.source_ports.clone(),
            p.dest_ports.clone(),
        ))
        .fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut si, mut di, mut sp, mut dp), (sim, dim, spm, dpm)| {
                si.push(sim);
                di.push(dim);
                sp.push(spm);
                dp.push(dpm);
                (si, di, sp, dp)
            },
        );

    // Collect all offsets
    let all_offsets: Vec<(u64, u64)> = partial_indexes
        .into_iter()
        .flat_map(|p| p.offsets)
        .collect();

    // Merge in parallel using rayon join for different index types
    let (bitmap_results, inverted_results) = rayon::join(
        || {
            rayon::join(
                || merge_bitmap_maps(actions_maps),
                || rayon::join(
                    || merge_bitmap_maps(protocols_maps),
                    || merge_bitmap_maps(interfaces_maps),
                ),
            )
        },
        || {
            rayon::join(
                || rayon::join(
                    || merge_string_maps(src_ip_maps),
                    || merge_string_maps(dst_ip_maps),
                ),
                || rayon::join(
                    || merge_port_maps(src_port_maps),
                    || merge_port_maps(dst_port_maps),
                ),
            )
        },
    );

    // Destructure bitmap results: (actions, (protocols, interfaces))
    let (merged_actions, (merged_protocols, merged_interfaces)) = bitmap_results;
    // Destructure inverted results: ((src_ips, dst_ips), (src_ports, dst_ports))
    let ((merged_src_ips, merged_dst_ips), (merged_src_ports, merged_dst_ports)) = inverted_results;

    // Build final indexes
    let bitmap_index = BitmapIndex::from_raw(merged_actions, merged_protocols, merged_interfaces);
    let inverted_index = InvertedIndex::from_raw(
        merged_src_ips,
        merged_dst_ips,
        merged_src_ports,
        merged_dst_ports,
    );

    // Build offset table (pre-allocate for performance)
    let mut offset_table = OffsetTable::with_capacity(total_entries as usize);
    for (entry_id, offset) in all_offsets {
        offset_table.set_offset(entry_id, offset);
    }

    let final_elapsed = start_time.elapsed().as_secs_f64();
    progress_callback(IndexProgress::new(file_size, file_size, final_elapsed));

    log::info!(
        "[PARALLEL] Complete: {} entries in {:.2}s ({:.0} entries/sec)",
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
    fn test_find_chunk_boundaries_simple() {
        let data = b"line1\nline2\nline3\n";
        let chunks = find_chunk_boundaries(data, 6);
        assert!(chunks.len() >= 1);
        assert_eq!(chunks[0].start, 0);
    }

    #[test]
    fn test_find_chunk_boundaries_large_chunk() {
        let data = b"line1\nline2\nline3\n";
        let chunks = find_chunk_boundaries(data, 1000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, data.len());
    }

    #[test]
    fn test_chunk_boundary_alignment() {
        let mut data = Vec::new();
        for i in 0..100 {
            data.extend_from_slice(format!("line{}\n", i).as_bytes());
        }

        let chunks = find_chunk_boundaries(&data, 50);

        for chunk in &chunks {
            if chunk.end < data.len() {
                assert_eq!(data[chunk.end - 1], b'\n');
            }
        }
    }

    #[test]
    fn test_merge_bitmap_maps() {
        let mut map1 = HashMap::new();
        let mut bitmap1 = RoaringBitmap::new();
        bitmap1.insert(1);
        bitmap1.insert(2);
        map1.insert("block".to_string(), bitmap1);

        let mut map2 = HashMap::new();
        let mut bitmap2 = RoaringBitmap::new();
        bitmap2.insert(3);
        bitmap2.insert(4);
        map2.insert("block".to_string(), bitmap2);

        let mut bitmap3 = RoaringBitmap::new();
        bitmap3.insert(5);
        map2.insert("pass".to_string(), bitmap3);

        let merged = merge_bitmap_maps(vec![map1, map2]);

        assert_eq!(merged.get("block").unwrap().len(), 4);
        assert_eq!(merged.get("pass").unwrap().len(), 1);
    }
}
