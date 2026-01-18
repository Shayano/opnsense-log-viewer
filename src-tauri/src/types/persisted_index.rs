use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::indexer::hybrid::HybridIndex;
use crate::types::log_entry::LogFormat;

/// Version number for index format evolution
/// Increment on breaking changes to serialization format
const INDEX_FORMAT_VERSION: u32 = 1;

/// Persisted index structure with integrity metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedIndex {
    /// Format version (for future compatibility)
    pub version: u32,

    /// SHA-256 hash of the serialized index content (for corruption detection)
    pub index_checksum: [u8; 32],

    /// SHA-256 hash of the source log file (for modification detection)
    pub source_file_hash: [u8; 32],

    /// Index creation timestamp
    pub created_at: DateTime<Utc>,

    /// Source file metadata
    pub source_metadata: SourceFileMetadata,

    /// The actual hybrid index (inverted + bitmap + offset table)
    pub hybrid_index: HybridIndex,
}

/// Metadata about the source log file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFileMetadata {
    pub file_path: String,
    pub file_size: u64,
    pub entry_count: u64,
    pub log_format: LogFormat,
}

impl PersistedIndex {
    /// Create a new PersistedIndex with default values
    pub fn new(
        source_file_hash: [u8; 32],
        source_metadata: SourceFileMetadata,
        hybrid_index: HybridIndex,
    ) -> Self {
        Self {
            version: INDEX_FORMAT_VERSION,
            index_checksum: [0; 32], // Calculated after serialization
            source_file_hash,
            created_at: Utc::now(),
            source_metadata,
            hybrid_index,
        }
    }

    /// Generate index filename from source file hash
    pub fn index_filename(source_hash: &[u8; 32]) -> String {
        format!("{}.idx", hex::encode(source_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_filename_format() {
        let hash = [1u8; 32];
        let filename = PersistedIndex::index_filename(&hash);
        assert!(filename.ends_with(".idx"));
        assert_eq!(filename.len(), 64 + 4); // 32 bytes hex (64 chars) + ".idx" (4 chars)
    }

    #[test]
    fn test_persisted_index_creation() {
        let source_hash = [42u8; 32];
        let metadata = SourceFileMetadata {
            file_path: "/test/log.txt".to_string(),
            file_size: 1024,
            entry_count: 100,
            log_format: LogFormat::RFC3164,
        };
        let hybrid_index = HybridIndex::new();

        let persisted = PersistedIndex::new(source_hash, metadata.clone(), hybrid_index);

        assert_eq!(persisted.version, INDEX_FORMAT_VERSION);
        assert_eq!(persisted.source_file_hash, source_hash);
        assert_eq!(persisted.source_metadata.file_size, 1024);
        assert_eq!(persisted.source_metadata.entry_count, 100);
    }
}
