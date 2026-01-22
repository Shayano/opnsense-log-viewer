use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
// Story 1.7: Removed sha2 import - using crate::storage::calculate_file_hash instead

use crate::indexer::inverted::InvertedIndex;
use crate::indexer::bitmap::BitmapIndex;
use crate::indexer::offset_table::OffsetTable;
use crate::indexer::progress::IndexProgress;
use crate::indexer::parallel::build_index_parallel;
use crate::indexer::streaming::build_index_streaming;
use crate::parser::{csv_filterlog, rfc3164, rfc5424};
use crate::types::log_entry::LogFormat;

/// Threshold for switching to parallel indexing (100 MB)
/// Files larger than this will use multi-threaded processing
const PARALLEL_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Threshold for switching to streaming indexing (1 GB)
/// Files larger than this will use batch processing to limit memory usage
/// Story 6.1: Memory-efficient indexing for large files
const STREAMING_THRESHOLD: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexMetadata {
    pub format: LogFormat,
    pub entry_count: u64,
    pub created_at: DateTime<Utc>,
    pub source_file_path: String,
    pub source_file_size: u64,
    pub source_file_hash: String,
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Indexation cancelled by user")]
    Cancelled,

    #[error("Memory limit exceeded: {0} MB used")]
    MemoryLimitExceeded(usize),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Story 6.3 AC4: Lock acquisition failure for concurrent access
    #[error("Lock error: {0}")]
    LockError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridIndex {
    inverted_index: InvertedIndex,
    bitmap_index: BitmapIndex,
    offset_table: OffsetTable,
    metadata: Option<IndexMetadata>,
    #[serde(skip)]
    #[serde(default = "default_cancellation_token")]
    cancellation_token: Arc<AtomicBool>,
}

fn default_cancellation_token() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

impl Default for HybridIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridIndex {
    pub fn new() -> Self {
        Self {
            inverted_index: InvertedIndex::new(),
            bitmap_index: BitmapIndex::new(),
            offset_table: OffsetTable::new(),
            metadata: None,
            cancellation_token: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create HybridIndex from a HotIndex (used for cache restoration)
    ///
    /// Story 6.4 Bug Fix: When loading from cache, we need to populate
    /// the HYBRID_INDEX with the cached data so queries work correctly.
    pub fn from_hot_index(
        hot: crate::indexer::tiered::HotIndex,
        metadata: Option<IndexMetadata>,
    ) -> Self {
        Self {
            inverted_index: hot.inverted_index,
            bitmap_index: hot.bitmap_index,
            offset_table: hot.offset_table,
            metadata,
            cancellation_token: Arc::new(AtomicBool::new(false)),
        }
    }

    // Story 1.7: Removed duplicate calculate_file_hash - use crate::storage::calculate_file_hash instead

    /// Build index from a log file with progress callback
    ///
    /// Automatically selects indexing strategy based on file size:
    /// - Files > 1 GB: Use STREAMING batch processing (memory-efficient, ~2GB peak)
    /// - Files > 100 MB: Use parallel multi-threaded processing (rayon + mmap)
    /// - Smaller files: Use sequential processing (lower overhead)
    ///
    /// Story 6.1: Added streaming mode for large files to reduce memory from 23GB to ~2GB
    pub fn build_index<P, F>(
        &mut self,
        file_path: P,
        format: LogFormat,
        progress_callback: F,
    ) -> Result<IndexMetadata, IndexError>
    where
        P: AsRef<Path>,
        F: FnMut(IndexProgress),
    {
        let file_path = file_path.as_ref();
        let file_size = std::fs::metadata(file_path)?.len();

        // Calculate SHA-256 hash of source file using optimized storage module (256KB buffer)
        // Story 1.7: Use unified hash implementation instead of duplicate
        let source_hash_bytes = crate::storage::calculate_file_hash(file_path)
            .map_err(|e| IndexError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        let source_hash = hex::encode(source_hash_bytes);

        // Choose indexing strategy based on file size
        // Story 6.1: Files > 1GB use streaming to limit memory to ~2GB
        if file_size > STREAMING_THRESHOLD {
            log::info!(
                "[PERF] File size {} MB > streaming threshold {} MB, using STREAMING indexing (memory-efficient)",
                file_size / (1024 * 1024),
                STREAMING_THRESHOLD / (1024 * 1024)
            );
            self.build_index_streaming(file_path, format, file_size, source_hash, progress_callback)
        } else if file_size > PARALLEL_THRESHOLD {
            log::info!(
                "[PERF] File size {} MB > parallel threshold {} MB, using PARALLEL indexing",
                file_size / (1024 * 1024),
                PARALLEL_THRESHOLD / (1024 * 1024)
            );
            self.build_index_parallel(file_path, format, file_size, source_hash, progress_callback)
        } else {
            log::info!(
                "[PERF] File size {} MB <= threshold {} MB, using SEQUENTIAL indexing",
                file_size / (1024 * 1024),
                PARALLEL_THRESHOLD / (1024 * 1024)
            );
            self.build_index_sequential(file_path, format, file_size, source_hash, progress_callback)
        }
    }

    /// Build index using streaming batch processing (memory-efficient for large files)
    /// Story 6.1: Processes in 100-chunk batches, persists to disk, frees memory
    fn build_index_streaming<P, F>(
        &mut self,
        file_path: P,
        format: LogFormat,
        file_size: u64,
        source_hash: String,
        progress_callback: F,
    ) -> Result<IndexMetadata, IndexError>
    where
        P: AsRef<Path>,
        F: FnMut(IndexProgress),
    {
        let (inverted, bitmap, offset_table, metadata) = build_index_streaming(
            file_path,
            format,
            file_size,
            source_hash,
            self.cancellation_token.clone(),
            progress_callback,
        )?;

        self.inverted_index = inverted;
        self.bitmap_index = bitmap;
        self.offset_table = offset_table;
        self.metadata = Some(metadata.clone());

        log::info!(
            "[MEM] HybridIndex::build_index_streaming: done entry_count={} memory_usage≈{} bytes",
            metadata.entry_count,
            self.memory_usage()
        );

        Ok(metadata)
    }

    /// Build index using parallel multi-threaded processing
    fn build_index_parallel<P, F>(
        &mut self,
        file_path: P,
        format: LogFormat,
        file_size: u64,
        source_hash: String,
        progress_callback: F,
    ) -> Result<IndexMetadata, IndexError>
    where
        P: AsRef<Path>,
        F: FnMut(IndexProgress),
    {
        let (inverted, bitmap, offset_table, metadata) = build_index_parallel(
            file_path,
            format,
            file_size,
            source_hash,
            self.cancellation_token.clone(),
            progress_callback,
        )?;

        self.inverted_index = inverted;
        self.bitmap_index = bitmap;
        self.offset_table = offset_table;
        self.metadata = Some(metadata.clone());

        log::info!(
            "[MEM] HybridIndex::build_index_parallel: done entry_count={} memory_usage≈{} bytes",
            metadata.entry_count,
            self.memory_usage()
        );

        Ok(metadata)
    }

    /// Build index using sequential single-threaded processing
    fn build_index_sequential<P, F>(
        &mut self,
        file_path: P,
        format: LogFormat,
        file_size: u64,
        source_hash: String,
        mut progress_callback: F,
    ) -> Result<IndexMetadata, IndexError>
    where
        P: AsRef<Path>,
        F: FnMut(IndexProgress),
    {
        let file_path = file_path.as_ref();

        // Build indexes by reading line-by-line to avoid loading the entire file into memory.
        // Performance optimization: Use 256KB buffer instead of default 8KB
        // This reduces syscalls by 32x on large files (e.g., 17GB: 65K vs 2M syscalls)
        let file = File::open(file_path)?;
        let reader = BufReader::with_capacity(256 * 1024, file);

        let mut line_number = 0u64;
        let mut entry_id = 0u64;
        let mut bytes_processed = 0u64;
        let start_time = std::time::Instant::now();
        let mut last_progress_time = start_time;

        for line_result in reader.lines() {
            line_number += 1;

            let line = match line_result {
                Ok(l) => l,
                Err(e) => return Err(IndexError::IoError(e)),
            };

            let line_bytes = (line.len() + 1) as u64;

            if line.trim().is_empty() {
                bytes_processed += line_bytes;
                continue;
            }

            let parse_result = match format {
                LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(&line, line_number, entry_id),
                LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(&line, line_number, entry_id),
                LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(&line, line_number, entry_id),
                LogFormat::Unknown => {
                    return Err(IndexError::ParseError("Unknown format".to_string()));
                }
            };

            match parse_result {
                Ok(entry) => {
                    if entry_id % 100 == 0 && self.cancellation_token.load(Ordering::Relaxed) {
                        return Err(IndexError::Cancelled);
                    }
                    if entry_id % 1000 == 0 {
                        let memory_usage = self.memory_usage();
                        if memory_usage > 500 * 1024 * 1024 {
                            return Err(IndexError::MemoryLimitExceeded(memory_usage / (1024 * 1024)));
                        }
                    }

                    self.inverted_index.add_entry(
                        entry.id,
                        entry.source_ip.as_deref(),
                        entry.dest_ip.as_deref(),
                        entry.source_port,
                        entry.dest_port,
                    );
                    self.bitmap_index.add_entry(
                        entry.id,
                        entry.action.as_deref(),
                        entry.protocol.as_deref(),
                        entry.interface.as_deref(),
                    );
                    self.offset_table.add_offset(entry.id, bytes_processed);

                    bytes_processed += line_bytes;
                    entry_id += 1;

                    let now = std::time::Instant::now();
                    // Performance: Reduce progress event frequency (2s instead of 500ms)
                    // to minimize IPC overhead on large files
                    if now.duration_since(last_progress_time).as_millis() >= 2000 {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        progress_callback(IndexProgress::new(bytes_processed, file_size, elapsed));
                        last_progress_time = now;
                    }
                }
                Err(_) => {
                    bytes_processed += line_bytes;
                }
            }
        }

        // Final progress
        progress_callback(IndexProgress::new(
            bytes_processed,
            file_size,
            start_time.elapsed().as_secs_f64(),
        ));

        let total_entries = entry_id;

        // Create metadata
        let metadata = IndexMetadata {
            format,
            entry_count: total_entries,
            created_at: Utc::now(),
            source_file_path: file_path.to_string_lossy().to_string(),
            source_file_size: file_size,
            source_file_hash: source_hash,
        };

        self.metadata = Some(metadata.clone());

        log::info!(
            "[MEM] HybridIndex::build_index_sequential: done entry_count={} memory_usage≈{} bytes",
            total_entries,
            self.memory_usage()
        );
        Ok(metadata)
    }

    /// Cancel indexation
    pub fn cancel(&self) {
        self.cancellation_token.store(true, Ordering::Relaxed);
    }

    /// Get approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.inverted_index.memory_usage()
            + self.bitmap_index.memory_usage()
            + self.offset_table.memory_usage()
    }

    /// Get metadata
    pub fn metadata(&self) -> Option<&IndexMetadata> {
        self.metadata.as_ref()
    }

    /// Get reference to inverted index
    pub fn inverted_index(&self) -> &InvertedIndex {
        &self.inverted_index
    }

    /// Get reference to bitmap index
    pub fn bitmap_index(&self) -> &BitmapIndex {
        &self.bitmap_index
    }

    /// Get reference to offset table
    pub fn offset_table(&self) -> &OffsetTable {
        &self.offset_table
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.metadata
            .as_ref()
            .map(|m| m.entry_count as usize)
            .unwrap_or(0)
    }

    /// Get source file hash
    pub fn source_file_hash(&self) -> Option<&str> {
        self.metadata.as_ref().map(|m| m.source_file_hash.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_usage_tracking() {
        let index = HybridIndex::new();
        let usage = index.memory_usage();
        assert!(usage < 500 * 1024 * 1024); // Less than 500 MB
    }
}
