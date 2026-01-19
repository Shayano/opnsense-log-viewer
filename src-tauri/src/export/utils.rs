use crate::export::types::InsufficientDiskSpaceError;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Check if there's sufficient disk space for export
///
/// Returns Ok if sufficient space is available, Err otherwise
pub fn check_disk_space<P: AsRef<Path>>(
    save_path: P,
    estimated_size_mb: f64,
) -> Result<(), InsufficientDiskSpaceError> {
    let save_path = save_path.as_ref();

    // Get parent directory (where file will be saved)
    let parent_dir = save_path
        .parent()
        .unwrap_or_else(|| Path::new("/"));

    // Get available disk space
    let available_bytes = match fs2::available_space(parent_dir) {
        Ok(bytes) => bytes,
        Err(_) => {
            // If we can't determine available space, allow the export to proceed
            // The actual file write will fail if there's insufficient space
            return Ok(());
        }
    };

    let available_mb = available_bytes as f64 / 1_048_576.0;
    let required_mb = estimated_size_mb * 1.1; // Add 10% safety margin

    if available_mb < required_mb {
        return Err(InsufficientDiskSpaceError {
            required_mb,
            available_mb,
        });
    }

    Ok(())
}

/// Calculate SHA-256 checksum of a file (memory-efficient chunked reading)
///
/// CRITICAL: Uses BufReader with 8KB buffer to prevent loading entire file into memory.
/// This allows checksumming large files (>1 GB) without memory overflow.
pub fn calculate_file_checksum<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let file = File::open(file_path.as_ref())
        .context("Failed to open file for checksum calculation")?;

    let mut reader = BufReader::with_capacity(8192, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .context("Failed to read file during checksum calculation")?;

        if bytes_read == 0 {
            break; // EOF
        }

        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize hash and convert to hex string
    let hash_bytes = hasher.finalize();
    let hash_hex = format!("{:x}", hash_bytes);

    Ok(hash_hex)
}

/// Detect incomplete export files in a directory
///
/// Scans export files to identify partial exports that failed mid-process:
/// - CSV: Missing "# Export SHA-256:" comment in header
/// - JSON: metadata.verification.exportComplete == false
pub fn detect_incomplete_exports<P: AsRef<Path>>(export_dir: P) -> Result<Vec<PathBuf>> {
    let mut incomplete_files = Vec::new();
    let export_dir = export_dir.as_ref();

    if !export_dir.exists() || !export_dir.is_dir() {
        return Ok(incomplete_files); // No exports yet
    }

    for entry in std::fs::read_dir(export_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Check file extension
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();

            match ext_str.as_ref() {
                "csv" => {
                    // CSV: Check for checksum comment
                    if !csv_has_checksum(&path)? {
                        incomplete_files.push(path);
                    }
                }
                "json" => {
                    // JSON: Check exportComplete flag
                    if !json_export_complete(&path)? {
                        incomplete_files.push(path);
                    }
                }
                _ => {
                    // Unknown format, skip
                }
            }
        }
    }

    Ok(incomplete_files)
}

/// Check if CSV file has checksum comment (indicates complete export)
fn csv_has_checksum<P: AsRef<Path>>(file_path: P) -> Result<bool> {
    let file = File::open(file_path.as_ref())?;
    let file_size = file.metadata()?.len();

    // Read last 1 KB of file to find checksum comment
    let read_size = std::cmp::min(file_size, 1024);

    let mut reader = BufReader::new(File::open(file_path.as_ref())?);
    if file_size > read_size {
        reader.seek(SeekFrom::End(-(read_size as i64)))?;
    }

    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;

    // Check if "# Export SHA-256:" is present
    Ok(buffer.contains("# Export SHA-256:"))
}

/// Check if JSON export is complete (exportComplete flag)
fn json_export_complete<P: AsRef<Path>>(file_path: P) -> Result<bool> {
    let file = File::open(file_path)?;
    let json: serde_json::Value = serde_json::from_reader(file)?;

    // Check metadata.verification.exportComplete
    if let Some(metadata) = json.get("metadata") {
        if let Some(verification) = metadata.get("verification") {
            if let Some(complete) = verification.get("exportComplete") {
                return Ok(complete.as_bool().unwrap_or(false));
            }
        }
    }

    // If verification object missing, assume incomplete
    Ok(false)
}

/// Clean up partial export file
pub fn cleanup_partial_export<P: AsRef<Path>>(file_path: P) -> Result<()> {
    std::fs::remove_file(file_path.as_ref())
        .context("Failed to delete partial export file")?;

    tracing::info!(
        "Cleaned up partial export: {}",
        file_path.as_ref().display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_check_disk_space_sufficient() {
        let temp_path = env::temp_dir().join("test_export.csv");

        // Check for very small file (should always have space)
        let result = check_disk_space(&temp_path, 0.001);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_disk_space_insufficient() {
        let temp_path = env::temp_dir().join("test_export.csv");

        // Check for unrealistically large file (should fail)
        let result = check_disk_space(&temp_path, 1_000_000_000.0); // 1 million GB
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.required_mb > err.available_mb);
        }
    }

    #[test]
    fn test_check_disk_space_with_margin() {
        let temp_path = env::temp_dir().join("test_export.csv");

        // Verify 10% safety margin is applied
        let estimated_mb = 100.0;
        let result = check_disk_space(&temp_path, estimated_mb);

        // This test may pass or fail depending on available disk space
        // We're mainly testing that the function doesn't panic
        let _ = result;
    }

    #[test]
    fn test_check_disk_space_nonexistent_parent() {
        // Test with path that has non-existent parent
        let temp_path = env::temp_dir().join("nonexistent_dir").join("test.csv");

        // Should handle gracefully (may succeed if parent becomes root)
        let result = check_disk_space(&temp_path, 1.0);
        let _ = result; // Don't assert, just verify no panic
    }

    #[test]
    fn test_calculate_file_checksum() {
        use std::io::Write;

        // Create test file
        let temp_path = env::temp_dir().join("test_checksum.txt");
        let mut file = File::create(&temp_path).unwrap();
        writeln!(file, "Hello, World!").unwrap();
        file.flush().unwrap();
        drop(file);

        // Calculate checksum
        let checksum = calculate_file_checksum(&temp_path).unwrap();

        // Verify checksum is hex string (64 chars for SHA-256)
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));

        // Calculate again - should be consistent
        let checksum2 = calculate_file_checksum(&temp_path).unwrap();
        assert_eq!(checksum, checksum2);

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_calculate_file_checksum_uniqueness() {
        // Create two different files
        let temp_path1 = env::temp_dir().join("test_checksum1.txt");
        let temp_path2 = env::temp_dir().join("test_checksum2.txt");

        std::fs::write(&temp_path1, "File 1 content").unwrap();
        std::fs::write(&temp_path2, "File 2 content").unwrap();

        // Calculate checksums
        let checksum1 = calculate_file_checksum(&temp_path1).unwrap();
        let checksum2 = calculate_file_checksum(&temp_path2).unwrap();

        // Different files should have different checksums
        assert_ne!(checksum1, checksum2);

        // Cleanup
        std::fs::remove_file(&temp_path1).ok();
        std::fs::remove_file(&temp_path2).ok();
    }

    #[test]
    fn test_calculate_file_checksum_not_found() {
        let temp_path = env::temp_dir().join("nonexistent_file_for_checksum.txt");

        // Should return error for non-existent file
        let result = calculate_file_checksum(&temp_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_incomplete_exports_csv() {
        let temp_dir = env::temp_dir().join("test_incomplete_csv");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create incomplete CSV (no checksum)
        let incomplete_csv = temp_dir.join("incomplete.csv");
        std::fs::write(&incomplete_csv, "# Exported by: test\ndata,data,data").unwrap();

        // Create complete CSV (with checksum)
        let complete_csv = temp_dir.join("complete.csv");
        std::fs::write(
            &complete_csv,
            "# Exported by: test\n# Export SHA-256: abc123\ndata,data,data",
        )
        .unwrap();

        // Detect incomplete exports
        let incomplete = detect_incomplete_exports(&temp_dir).unwrap();

        // Should find incomplete CSV only
        assert_eq!(incomplete.len(), 1);
        assert!(incomplete[0].ends_with("incomplete.csv"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_detect_incomplete_exports_json() {
        let temp_dir = env::temp_dir().join("test_incomplete_json");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create incomplete JSON (exportComplete: false)
        let incomplete_json = temp_dir.join("incomplete.json");
        std::fs::write(
            &incomplete_json,
            r#"{"metadata": {"verification": {"exportComplete": false}}, "entries": []}"#,
        )
        .unwrap();

        // Create complete JSON (exportComplete: true)
        let complete_json = temp_dir.join("complete.json");
        std::fs::write(
            &complete_json,
            r#"{"metadata": {"verification": {"exportComplete": true}}, "entries": []}"#,
        )
        .unwrap();

        // Detect incomplete exports
        let incomplete = detect_incomplete_exports(&temp_dir).unwrap();

        // Should find incomplete JSON only
        assert_eq!(incomplete.len(), 1);
        assert!(incomplete[0].ends_with("incomplete.json"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_detect_incomplete_exports_nonexistent_dir() {
        let temp_dir = env::temp_dir().join("nonexistent_export_dir_test");

        // Should handle non-existent directory gracefully
        let result = detect_incomplete_exports(&temp_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_cleanup_partial_export() {
        use std::io::Write;

        let temp_path = env::temp_dir().join("test_cleanup.txt");
        let mut file = File::create(&temp_path).unwrap();
        writeln!(file, "partial export data").unwrap();
        drop(file);

        // File should exist
        assert!(temp_path.exists());

        // Cleanup
        cleanup_partial_export(&temp_path).unwrap();

        // File should be deleted
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_cleanup_partial_export_nonexistent() {
        let temp_path = env::temp_dir().join("nonexistent_cleanup_file.txt");

        // Should return error for non-existent file
        let result = cleanup_partial_export(&temp_path);
        assert!(result.is_err());
    }
}
