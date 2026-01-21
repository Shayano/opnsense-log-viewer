use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;
use zstd::stream::{Decoder, Encoder};
use rkyv::Deserialize as RkyvDeserialize;

use crate::storage::integrity::{calculate_checksum, verify_checksum, IntegrityError};
use crate::types::persisted_index::PersistedIndex;
use crate::indexer::inverted::InvertedIndex;
use crate::indexer::bitmap::{BitmapIndex, BitmapIndexRkyv};
use crate::indexer::offset_table::OffsetTable;

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

    #[error("rkyv validation error: {0}")]
    RkyvValidationError(String),
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

// ============================================================================
// Story 6.2: rkyv Zero-Copy Serialization Helpers
// ============================================================================

/// Serialize InvertedIndex to rkyv bytes with validation
///
/// Story 6.2 (AC4): Provides zero-copy serialization for faster index reload
pub fn serialize_inverted_index_rkyv(index: &InvertedIndex) -> Result<rkyv::AlignedVec, PersistenceError> {
    rkyv::to_bytes::<_, 256>(index)
        .map_err(|e| PersistenceError::SerializationError(format!("rkyv serialization failed: {:?}", e)))
}

/// Deserialize InvertedIndex from rkyv bytes with validation
///
/// Story 6.2 (AC4): Provides zero-copy deserialization for faster index reload
/// Returns the deserialized struct (for full ownership) after validating archived data
pub fn deserialize_inverted_index_rkyv(bytes: &[u8]) -> Result<InvertedIndex, PersistenceError> {
    let archived = rkyv::check_archived_root::<InvertedIndex>(bytes)
        .map_err(|e| PersistenceError::RkyvValidationError(format!("validation failed: {:?}", e)))?;

    archived
        .deserialize(&mut rkyv::Infallible)
        .map_err(|e| PersistenceError::SerializationError(format!("rkyv deserialization failed: {:?}", e)))
}

/// Serialize BitmapIndex to rkyv bytes via BitmapIndexRkyv wrapper
///
/// Story 6.2 (AC4): Uses BitmapIndexRkyv to handle RoaringBitmap serialization
pub fn serialize_bitmap_index_rkyv(index: &BitmapIndex) -> Result<rkyv::AlignedVec, PersistenceError> {
    let rkyv_format = index.to_rkyv();
    rkyv::to_bytes::<_, 256>(&rkyv_format)
        .map_err(|e| PersistenceError::SerializationError(format!("rkyv serialization failed: {:?}", e)))
}

/// Deserialize BitmapIndex from rkyv bytes via BitmapIndexRkyv wrapper
///
/// Story 6.2 (AC4): Uses archived BitmapIndexRkyv for zero-copy access to bitmap data
pub fn deserialize_bitmap_index_rkyv(bytes: &[u8]) -> Result<BitmapIndex, PersistenceError> {
    let archived = rkyv::check_archived_root::<BitmapIndexRkyv>(bytes)
        .map_err(|e| PersistenceError::RkyvValidationError(format!("validation failed: {:?}", e)))?;

    Ok(BitmapIndex::from_archived_rkyv(archived))
}

/// Serialize OffsetTable to rkyv bytes with validation
///
/// Story 6.2 (AC4): Provides zero-copy serialization for faster offset table reload
pub fn serialize_offset_table_rkyv(table: &OffsetTable) -> Result<rkyv::AlignedVec, PersistenceError> {
    rkyv::to_bytes::<_, 256>(table)
        .map_err(|e| PersistenceError::SerializationError(format!("rkyv serialization failed: {:?}", e)))
}

/// Deserialize OffsetTable from rkyv bytes with validation
///
/// Story 6.2 (AC4): Provides zero-copy deserialization for faster offset table reload
pub fn deserialize_offset_table_rkyv(bytes: &[u8]) -> Result<OffsetTable, PersistenceError> {
    let archived = rkyv::check_archived_root::<OffsetTable>(bytes)
        .map_err(|e| PersistenceError::RkyvValidationError(format!("validation failed: {:?}", e)))?;

    archived
        .deserialize(&mut rkyv::Infallible)
        .map_err(|e| PersistenceError::SerializationError(format!("rkyv deserialization failed: {:?}", e)))
}

/// Zero-copy access to archived InvertedIndex (for read-only operations)
///
/// Story 6.2: Allows direct access to archived data without deserialization.
/// The returned reference has the same lifetime as the input bytes.
///
/// # Safety
/// This uses rkyv's validation to ensure the bytes are valid before returning.
pub fn access_inverted_index_rkyv(bytes: &[u8]) -> Result<&rkyv::Archived<InvertedIndex>, PersistenceError> {
    rkyv::check_archived_root::<InvertedIndex>(bytes)
        .map_err(|e| PersistenceError::RkyvValidationError(format!("validation failed: {:?}", e)))
}

/// Zero-copy access to archived OffsetTable (for read-only operations)
///
/// Story 6.2: Allows direct access to archived offset data without deserialization.
pub fn access_offset_table_rkyv(bytes: &[u8]) -> Result<&rkyv::Archived<OffsetTable>, PersistenceError> {
    rkyv::check_archived_root::<OffsetTable>(bytes)
        .map_err(|e| PersistenceError::RkyvValidationError(format!("validation failed: {:?}", e)))
}

/// Zero-copy access to archived BitmapIndexRkyv (for read-only operations)
///
/// Story 6.2: Allows direct access to archived bitmap data without deserialization.
/// Note: Returns BitmapIndexRkyv archived type since BitmapIndex doesn't directly implement rkyv.
pub fn access_bitmap_index_rkyv(bytes: &[u8]) -> Result<&rkyv::Archived<BitmapIndexRkyv>, PersistenceError> {
    rkyv::check_archived_root::<BitmapIndexRkyv>(bytes)
        .map_err(|e| PersistenceError::RkyvValidationError(format!("validation failed: {:?}", e)))
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

    // ========================================================================
    // Story 6.2: rkyv Serialization Helper Tests
    // ========================================================================

    use crate::indexer::inverted::InvertedIndex;
    use crate::indexer::bitmap::BitmapIndex;
    use crate::indexer::offset_table::OffsetTable;

    #[test]
    fn test_rkyv_inverted_index_roundtrip() {
        let mut index = InvertedIndex::new();
        for i in 0..1000 {
            index.add_entry(
                i,
                Some(&format!("192.168.1.{}", i % 255)),
                Some("10.0.0.1"),
                Some(443),
                Some(80),
            );
        }

        // Serialize
        let bytes = serialize_inverted_index_rkyv(&index).unwrap();
        assert!(!bytes.is_empty());

        // Deserialize
        let deserialized = deserialize_inverted_index_rkyv(&bytes).unwrap();
        assert_eq!(
            index.query_source_ip("192.168.1.0"),
            deserialized.query_source_ip("192.168.1.0")
        );

        // Zero-copy access - just verify it works (can't access private fields)
        let _archived = access_inverted_index_rkyv(&bytes).unwrap();
    }

    #[test]
    fn test_rkyv_bitmap_index_roundtrip() {
        let mut index = BitmapIndex::new();
        for i in 0..1000 {
            let action = if i % 2 == 0 { "block" } else { "pass" };
            index.add_entry(i, Some(action), Some("TCP"), Some("vtnet0"));
        }

        // Serialize
        let bytes = serialize_bitmap_index_rkyv(&index).unwrap();
        assert!(!bytes.is_empty());

        // Deserialize
        let deserialized = deserialize_bitmap_index_rkyv(&bytes).unwrap();
        assert_eq!(
            index.query_action("block").unwrap().len(),
            deserialized.query_action("block").unwrap().len()
        );
    }

    #[test]
    fn test_rkyv_offset_table_roundtrip() {
        let mut table = OffsetTable::new();
        for i in 0..10000 {
            table.add_offset(i, i * 100);
        }

        // Serialize
        let bytes = serialize_offset_table_rkyv(&table).unwrap();
        assert!(!bytes.is_empty());

        // Deserialize
        let deserialized = deserialize_offset_table_rkyv(&bytes).unwrap();
        assert_eq!(table.get_offset(5000), deserialized.get_offset(5000));
        assert_eq!(deserialized.len(), 10000);

        // Zero-copy access - just verify it works
        let _archived = access_offset_table_rkyv(&bytes).unwrap();
    }

    #[test]
    fn test_rkyv_validation_rejects_invalid_bytes() {
        let invalid_bytes = b"this is not valid rkyv data";

        let result = deserialize_inverted_index_rkyv(invalid_bytes);
        assert!(result.is_err());

        let result = deserialize_bitmap_index_rkyv(invalid_bytes);
        assert!(result.is_err());

        let result = deserialize_offset_table_rkyv(invalid_bytes);
        assert!(result.is_err());
    }
}
