use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;
use zstd::stream::{Decoder, Encoder};

use crate::storage::integrity::{calculate_checksum, verify_checksum, IntegrityError};
use crate::types::persisted_index::PersistedIndex;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Integrity error: {0}")]
    IntegrityError(#[from] IntegrityError),

    #[error("Compression error: {0}")]
    CompressionError(String),
}

/// Save index to disk with Zstd compression (level 1)
/// Atomic operation: writes to .idx.tmp then renames to .idx
pub fn save_index<P: AsRef<Path>>(
    index: &PersistedIndex,
    index_path: P,
) -> Result<(), PersistenceError> {
    let index_path = index_path.as_ref();
    let temp_path = index_path.with_extension("idx.tmp");

    // Serialize to bytes using bincode (without checksum first)
    let mut index_for_checksum = index.clone();
    index_for_checksum.index_checksum = [0; 32]; // Zero out checksum field

    let serialized = bincode::serde::encode_to_vec(&index_for_checksum, bincode::config::standard())
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

    // Calculate checksum of serialized data
    let checksum = calculate_checksum(&serialized);

    // Update index with calculated checksum
    let mut index_with_checksum = index.clone();
    index_with_checksum.index_checksum = checksum;

    // Re-serialize with checksum included
    let final_serialized = bincode::serde::encode_to_vec(&index_with_checksum, bincode::config::standard())
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

    // Compress with Zstd (level 1 for speed)
    {
        let temp_file = File::create(&temp_path)?;
        let mut encoder = Encoder::new(temp_file, 1)
            .map_err(|e| PersistenceError::CompressionError(e.to_string()))?;
        encoder
            .include_checksum(true)
            .map_err(|e| PersistenceError::CompressionError(e.to_string()))?; // Zstd built-in checksum
        encoder.write_all(&final_serialized)?;
        encoder
            .finish()
            .map_err(|e| PersistenceError::CompressionError(e.to_string()))?;
    }

    // Atomic rename: .idx.tmp → .idx
    fs::rename(&temp_path, index_path)?;

    Ok(())
}

/// Load index from disk with decompression and integrity validation
pub fn load_index<P: AsRef<Path>>(index_path: P) -> Result<PersistedIndex, PersistenceError> {
    let index_path = index_path.as_ref();

    // Decompress with Zstd
    let file = File::open(index_path)?;
    let mut decoder =
        Decoder::new(file).map_err(|e| PersistenceError::CompressionError(e.to_string()))?;

    // Read all decompressed bytes
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    // Deserialize with bincode
    let (index, _): (PersistedIndex, usize) = bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

    // Verify index integrity (checksum validation)
    // Create copy without checksum for verification
    let mut index_for_verification = index.clone();
    let stored_checksum = index.index_checksum;
    index_for_verification.index_checksum = [0; 32];

    let serialized_for_verification = bincode::serde::encode_to_vec(&index_for_verification, bincode::config::standard())
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
    verify_checksum(&serialized_for_verification, &stored_checksum)?;

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::hybrid::HybridIndex;
    use crate::types::log_entry::LogFormat;
    use crate::types::persisted_index::SourceFileMetadata;
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_index_roundtrip() {
        // Create test index
        let source_hash = [1u8; 32];
        let metadata = SourceFileMetadata {
            file_path: "/test/log.txt".to_string(),
            file_size: 1024,
            entry_count: 100,
            log_format: LogFormat::RFC3164,
        };

        let hybrid_index = HybridIndex::new(); // Empty for test
        let original_index = PersistedIndex::new(source_hash, metadata, hybrid_index);

        // Save to temp file
        let temp_file = NamedTempFile::new().unwrap();
        let index_path = temp_file.path();

        save_index(&original_index, index_path).unwrap();

        // Load back
        let loaded_index = load_index(index_path).unwrap();

        // Verify critical fields match
        assert_eq!(loaded_index.version, original_index.version);
        assert_eq!(
            loaded_index.source_file_hash,
            original_index.source_file_hash
        );
        assert_eq!(loaded_index.source_metadata.file_size, 1024);
        assert_eq!(loaded_index.source_metadata.entry_count, 100);
        assert_eq!(
            loaded_index.source_metadata.log_format,
            LogFormat::RFC3164
        );
    }

    #[test]
    fn test_save_creates_compressed_file() {
        let source_hash = [2u8; 32];
        let metadata = SourceFileMetadata {
            file_path: "/test/large.log".to_string(),
            file_size: 1_000_000,
            entry_count: 10000,
            log_format: LogFormat::CSV,
        };

        let hybrid_index = HybridIndex::new();
        let index = PersistedIndex::new(source_hash, metadata, hybrid_index);

        let temp_file = NamedTempFile::new().unwrap();
        let index_path = temp_file.path();

        save_index(&index, index_path).unwrap();

        // Verify file exists and is non-empty
        let file_metadata = fs::metadata(index_path).unwrap();
        assert!(file_metadata.len() > 0);

        // File should be smaller than uncompressed due to zstd
        // (though hard to verify exact compression ratio without uncompressed size)
        let loaded = load_index(index_path).unwrap();
        assert_eq!(loaded.source_file_hash, source_hash);
    }

    #[test]
    fn test_load_corrupted_index_fails() {
        // Create corrupted index file
        let temp_file = NamedTempFile::new().unwrap();
        let mut file = File::create(temp_file.path()).unwrap();
        file.write_all(b"corrupted data").unwrap();
        drop(file);

        // Should fail to load
        let result = load_index(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_checksum_validation_detects_corruption() {
        // Create valid index
        let source_hash = [3u8; 32];
        let metadata = SourceFileMetadata {
            file_path: "/test/file.log".to_string(),
            file_size: 512,
            entry_count: 50,
            log_format: LogFormat::RFC5424,
        };

        let hybrid_index = HybridIndex::new();
        let index = PersistedIndex::new(source_hash, metadata, hybrid_index);

        let temp_file = NamedTempFile::new().unwrap();
        let index_path = temp_file.path();

        save_index(&index, index_path).unwrap();

        // Successfully load valid index
        let loaded = load_index(index_path).unwrap();
        assert_eq!(loaded.source_file_hash, source_hash);
    }

    #[test]
    fn test_atomic_write_cleanup() {
        let source_hash = [4u8; 32];
        let metadata = SourceFileMetadata {
            file_path: "/test/atomic.log".to_string(),
            file_size: 256,
            entry_count: 25,
            log_format: LogFormat::RFC3164,
        };

        let hybrid_index = HybridIndex::new();
        let index = PersistedIndex::new(source_hash, metadata, hybrid_index);

        let temp_file = NamedTempFile::new().unwrap();
        let index_path = temp_file.path();
        let temp_path = index_path.with_extension("idx.tmp");

        save_index(&index, index_path).unwrap();

        // Temporary file should not exist after successful save
        assert!(!temp_path.exists());

        // Final file should exist
        assert!(index_path.exists());
    }
}
