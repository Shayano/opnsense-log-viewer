use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntegrityError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
}

/// Buffer size for file hash calculation (256KB for optimal I/O performance)
/// Story 1.7: Increased from 4KB to 256KB for ~64x fewer I/O operations on large files
pub const HASH_BUFFER_SIZE: usize = 262144; // 256KB

/// Calculate SHA-256 hash of a file using streaming (256KB chunks)
/// Essential for large files (30GB) to avoid loading entire file into memory
///
/// # Performance
/// - 256KB buffer reduces I/O operations by ~64x compared to 4KB
/// - For 17GB file: 68K iterations vs 4.4M iterations
pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<[u8; 32], IntegrityError> {
    let file = File::open(file_path)?;
    // Story 1.7 Code Review: Use single buffer (File directly, not BufReader + vec)
    // This avoids double buffering and reduces memory from 512KB to 256KB
    let mut reader = file;
    let mut hasher = Sha256::new();

    // Read in 256KB chunks - optimized for large file performance
    let mut buffer = vec![0u8; HASH_BUFFER_SIZE];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // EOF reached
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    Ok(hash)
}

/// Calculate SHA-256 hash of serialized bytes (for index integrity)
pub fn calculate_checksum(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    hash
}

/// Verify index checksum matches expected value
pub fn verify_checksum(data: &[u8], expected: &[u8; 32]) -> Result<(), IntegrityError> {
    let actual = calculate_checksum(data);

    if &actual != expected {
        return Err(IntegrityError::ChecksumMismatch {
            expected: hex::encode(expected),
            actual: hex::encode(actual),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_file_hash_streaming() {
        // Create temp file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content for hashing").unwrap();

        let hash = calculate_file_hash(temp_file.path()).unwrap();

        // Verify hash is non-zero
        assert_ne!(hash, [0u8; 32]);

        // Hash should be deterministic
        let hash2 = calculate_file_hash(temp_file.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_calculate_file_hash_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let hash = calculate_file_hash(temp_file.path()).unwrap();
        assert_ne!(hash, [0u8; 32]); // Even empty file has a hash
    }

    #[test]
    fn test_calculate_file_hash_large_content() {
        // Create file larger than 4KB buffer to test streaming
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_content = vec![b'A'; 10000]; // 10KB
        temp_file.write_all(&large_content).unwrap();

        let hash = calculate_file_hash(temp_file.path()).unwrap();
        assert_ne!(hash, [0u8; 32]);

        // Verify deterministic
        let hash2 = calculate_file_hash(temp_file.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_calculate_checksum() {
        let data = b"test data";
        let checksum = calculate_checksum(data);

        // Checksum should be non-zero
        assert_ne!(checksum, [0u8; 32]);

        // Same data produces same checksum
        let checksum2 = calculate_checksum(data);
        assert_eq!(checksum, checksum2);

        // Different data produces different checksum
        let different_data = b"different data";
        let checksum3 = calculate_checksum(different_data);
        assert_ne!(checksum, checksum3);
    }

    #[test]
    fn test_verify_checksum_success() {
        let data = b"test data";
        let checksum = calculate_checksum(data);

        assert!(verify_checksum(data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_checksum_mismatch() {
        let data = b"test data";
        let wrong_checksum = [0u8; 32];

        let result = verify_checksum(data, &wrong_checksum);
        assert!(result.is_err());

        if let Err(IntegrityError::ChecksumMismatch { expected, actual }) = result {
            assert_eq!(expected, hex::encode(wrong_checksum));
            assert_ne!(expected, actual);
        } else {
            panic!("Expected ChecksumMismatch error");
        }
    }

    #[test]
    fn test_verify_checksum_deterministic() {
        let data = b"consistent test data";
        let checksum = calculate_checksum(data);

        // Verify multiple times - should always succeed
        for _ in 0..10 {
            assert!(verify_checksum(data, &checksum).is_ok());
        }
    }

    #[test]
    fn test_hash_buffer_size_is_256kb() {
        // Story 1.7: Verify buffer size constant is 256KB for optimal I/O performance
        assert_eq!(HASH_BUFFER_SIZE, 262144, "Buffer size should be 256KB (262144 bytes)");
        assert_eq!(HASH_BUFFER_SIZE, 256 * 1024, "Buffer size should be exactly 256 * 1024 bytes");
    }

    #[test]
    fn test_calculate_file_hash_large_content_with_optimized_buffer() {
        // Story 1.7: Test with content larger than 256KB to ensure streaming works
        let mut temp_file = NamedTempFile::new().unwrap();

        // Create file larger than buffer size (512KB > 256KB)
        let large_content = vec![b'X'; 512 * 1024];
        temp_file.write_all(&large_content).unwrap();

        let hash = calculate_file_hash(temp_file.path()).unwrap();
        assert_ne!(hash, [0u8; 32]);

        // Verify deterministic - same content always produces same hash
        let hash2 = calculate_file_hash(temp_file.path()).unwrap();
        assert_eq!(hash, hash2);
    }
}
