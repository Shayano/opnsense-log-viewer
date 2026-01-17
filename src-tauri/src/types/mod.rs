use serde::{Deserialize, Serialize};

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
}

/// File metadata (size, etc.)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
}
