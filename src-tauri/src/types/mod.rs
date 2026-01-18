pub mod log_entry;
pub mod persisted_index;

use serde::{Deserialize, Serialize};

// Re-export for convenience
pub use persisted_index::{PersistedIndex, SourceFileMetadata};

/// Metadata about the indexed file
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndexMetadata {
    /// SHA-256 hash of the source file
    pub source_file_hash: String,
    /// Number of log entries indexed
    pub entry_count: u64,
    /// Detected log format: "RFC3164" | "RFC5424" | "CSV" | "UNKNOWN"
    pub format: String,
    /// Size of the index file in bytes
    pub index_size_bytes: u64,
    /// ISO 8601 timestamp when the index was created
    pub created_at: String,
    /// Parsing statistics (total lines, skipped malformed, etc.)
    pub parsing_stats: Option<ParsingStats>,
}

/// Parsing statistics for transparency
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParsingStats {
    /// Total lines processed
    pub total_lines: u64,
    /// Successfully parsed entries
    pub parsed_successfully: u64,
    /// Skipped malformed lines
    pub skipped_malformed: u64,
}

/// File metadata (size, etc.)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
}
