use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::Read;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use sha2::{Sha256, Digest};

use crate::indexer::inverted::InvertedIndex;
use crate::indexer::bitmap::BitmapIndex;
use crate::indexer::offset_table::OffsetTable;
use crate::indexer::progress::IndexProgress;
use crate::parser::parse_file_streaming;
use crate::types::log_entry::LogFormat;

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

    /// Calculate SHA-256 hash of a file
    fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String, IndexError> {
        let mut file = std::fs::File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Build index from a log file with progress callback
    pub fn build_index<P, F>(
        &mut self,
        file_path: P,
        format: LogFormat,
        mut progress_callback: F,
    ) -> Result<IndexMetadata, IndexError>
    where
        P: AsRef<Path>,
        F: FnMut(IndexProgress),
    {
        let file_path = file_path.as_ref();
        let file_size = std::fs::metadata(file_path)?.len();

        // Calculate SHA-256 hash of source file
        let source_hash = Self::calculate_file_hash(file_path)?;

        // Parse file and build indexes
        let (entries, _parse_stats) = parse_file_streaming(file_path, format)
            .map_err(|e| IndexError::ParseError(e.to_string()))?;

        let total_entries = entries.len() as u64;
        let mut bytes_processed = 0u64;
        let start_time = std::time::Instant::now();
        let mut last_progress_time = start_time;

        for (idx, entry) in entries.into_iter().enumerate() {
            // Check cancellation every 100 entries
            if idx % 100 == 0 && self.cancellation_token.load(Ordering::Relaxed) {
                return Err(IndexError::Cancelled);
            }

            // Check memory usage every 1000 entries
            if idx % 1000 == 0 {
                let memory_usage = self.memory_usage();
                if memory_usage > 500 * 1024 * 1024 {
                    return Err(IndexError::MemoryLimitExceeded(memory_usage / (1024 * 1024)));
                }
            }

            // Add to inverted index (high-cardinality fields)
            self.inverted_index.add_entry(
                entry.id,
                entry.source_ip.as_deref(),
                entry.dest_ip.as_deref(),
                entry.source_port,
                entry.dest_port,
            );

            // Add to bitmap index (low-cardinality fields)
            self.bitmap_index.add_entry(
                entry.id,
                entry.action.as_deref(),
                entry.protocol.as_deref(),
                entry.interface.as_deref(),
            );

            // Add offset to offset table (raw line byte offset)
            // NOTE: In real implementation, would track actual file offsets during parsing
            // For now, using estimated offset based on average line size
            let estimated_offset = (idx as u64 * file_size) / total_entries;
            self.offset_table.add_offset(entry.id, estimated_offset);

            // Update progress every 500ms or at completion
            bytes_processed += entry.raw_line.len() as u64;
            let now = std::time::Instant::now();
            let is_complete = idx == total_entries as usize - 1;

            if now.duration_since(last_progress_time).as_millis() >= 500 || is_complete {
                let elapsed = start_time.elapsed().as_secs_f64();
                let progress = IndexProgress::new(
                    bytes_processed,
                    file_size,
                    elapsed,
                );
                progress_callback(progress);
                last_progress_time = now;
            }
        }

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
