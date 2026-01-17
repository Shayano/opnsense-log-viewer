use crate::types::{FileMetadata, IndexMetadata};
use crate::parser::{detect_format, parse_file_streaming};
use crate::types::log_entry::LogFormat;
use std::fs;
use std::path::Path;

/// Get file metadata (size)
#[tauri::command]
pub fn get_file_metadata(file_path: String) -> Result<FileMetadata, String> {
    let metadata = fs::metadata(&file_path)
        .map_err(|e| format!("Failed to read file metadata: {}. Please check the file path.", e))?;

    Ok(FileMetadata {
        size: metadata.len(),
    })
}

/// Index a log file
///
/// This command validates the file path, checks permissions, and eventually triggers
/// the indexation process (full implementation in Story 1.3).
///
/// # Arguments
/// * `file_path` - Path to the log file to index
/// * `compression_level` - Zstd compression level (1 recommended for speed)
///
/// # Returns
/// * `Ok(IndexMetadata)` - Metadata about the indexed file
/// * `Err(String)` - Error message if indexation fails
#[tauri::command]
pub async fn index_file(
    file_path: String,
    compression_level: u8,
) -> Result<IndexMetadata, String> {
    // 1. Validate file path exists
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err("File not found. Please ensure the file path is correct.".to_string());
    }

    if !path.is_file() {
        return Err("Path is not a file.".to_string());
    }

    // 2. Check read permissions by attempting to read metadata
    match fs::metadata(path) {
        Ok(_) => {
            // File is accessible
        }
        Err(e) => {
            return Err(format!(
                "Failed to access file: {}. Please check file permissions and try again.",
                e
            ));
        }
    }

    // 3. Validate no path traversal attacks
    let canonical_path = path.canonicalize()
        .map_err(|e| format!("Invalid file path: {}. Please check the path and try again.", e))?;

    // Ensure the path is absolute and doesn't contain suspicious patterns
    if !canonical_path.is_absolute() {
        return Err("Invalid file path: path must be absolute.".to_string());
    }

    // 4. Attempt to open file to verify read permissions
    match fs::File::open(&canonical_path) {
        Ok(_) => {
            // File can be read
        }
        Err(e) => {
            return Err(format!(
                "Failed to open file: {}. Please check file permissions and try again.",
                e
            ));
        }
    }

    // 5. Validate compression level
    if compression_level > 22 {
        return Err("Invalid compression level. Must be between 0 and 22.".to_string());
    }

    // 6. Detect log format
    let detected_format = detect_format(&canonical_path)
        .map_err(|e| format!("Failed to detect log format: {}. Please select format manually or check file contents.", e))?;

    // Convert LogFormat enum to string for IndexMetadata
    let format_str = match detected_format {
        LogFormat::RFC3164 => "RFC3164",
        LogFormat::RFC5424 => "RFC5424",
        LogFormat::CSV => "CSV",
        LogFormat::Unknown => {
            return Err("Unable to auto-detect log format. Please select format manually.".to_string());
        }
    };

    // 7. Parse log file with streaming parser
    let (entries, stats) = parse_file_streaming(&canonical_path, detected_format)
        .map_err(|e| format!("Failed to parse log file: {}", e))?;

    // Log parsing statistics
    log::info!(
        "Parsed {} entries successfully, skipped {} malformed lines from {} total lines",
        stats.parsed_successfully,
        stats.skipped_malformed,
        stats.total_lines
    );

    // TODO Story 1.3: Calculate SHA-256 hash of source file
    // TODO Story 1.3: Create index with entries and persist to disk
    // For now, return metadata with parsed entry count
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(IndexMetadata {
        source_file_hash: "pending".to_string(), // Will be calculated in Story 1.3
        entry_count: entries.len() as u64,
        format: format_str.to_string(),
        index_size_bytes: 0, // Will be calculated in Story 1.3
        created_at,
        parsing_stats: Some(crate::types::ParsingStats {
            total_lines: stats.total_lines,
            parsed_successfully: stats.parsed_successfully,
            skipped_malformed: stats.skipped_malformed,
        }),
    })
}

/// Index a log file with manually specified format
///
/// This command is used when auto-detection fails and the user manually selects the format.
///
/// # Arguments
/// * `file_path` - Path to the log file to index
/// * `format` - Log format as string: "RFC3164", "RFC5424", or "CSV"
/// * `compression_level` - Zstd compression level (1 recommended for speed)
///
/// # Returns
/// * `Ok(IndexMetadata)` - Metadata about the indexed file
/// * `Err(String)` - Error message if indexation fails
#[tauri::command]
pub async fn index_file_with_format(
    file_path: String,
    format: String,
    compression_level: u8,
) -> Result<IndexMetadata, String> {
    // 1. Validate file path (same as index_file)
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err("File not found. Please ensure the file path is correct.".to_string());
    }

    if !path.is_file() {
        return Err("Path is not a file.".to_string());
    }

    let canonical_path = path.canonicalize()
        .map_err(|e| format!("Invalid file path: {}. Please check the path and try again.", e))?;

    if !canonical_path.is_absolute() {
        return Err("Invalid file path: path must be absolute.".to_string());
    }

    match fs::File::open(&canonical_path) {
        Ok(_) => {},
        Err(e) => {
            return Err(format!(
                "Failed to open file: {}. Please check file permissions and try again.",
                e
            ));
        }
    }

    // 2. Validate compression level
    if compression_level > 22 {
        return Err("Invalid compression level. Must be between 0 and 22.".to_string());
    }

    // 3. Parse format string to LogFormat enum
    let log_format = match format.as_str() {
        "RFC3164" => LogFormat::RFC3164,
        "RFC5424" => LogFormat::RFC5424,
        "CSV" => LogFormat::CSV,
        _ => {
            return Err(format!(
                "Invalid format '{}'. Must be 'RFC3164', 'RFC5424', or 'CSV'.",
                format
            ));
        }
    };

    // 4. Parse log file with specified format
    let (entries, stats) = parse_file_streaming(&canonical_path, log_format)
        .map_err(|e| format!("Failed to parse log file: {}", e))?;

    // Log parsing statistics
    log::info!(
        "Parsed {} entries successfully, skipped {} malformed lines from {} total lines (format: {})",
        stats.parsed_successfully,
        stats.skipped_malformed,
        stats.total_lines,
        format
    );

    // TODO Story 1.3: Calculate SHA-256 hash of source file
    // TODO Story 1.3: Create index with entries and persist to disk
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(IndexMetadata {
        source_file_hash: "pending".to_string(),
        entry_count: entries.len() as u64,
        format,
        index_size_bytes: 0,
        created_at,
        parsing_stats: Some(crate::types::ParsingStats {
            total_lines: stats.total_lines,
            parsed_successfully: stats.parsed_successfully,
            skipped_malformed: stats.skipped_malformed,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_get_file_metadata_with_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let result = get_file_metadata(file_path.to_string_lossy().to_string());

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.size, 12); // "test content" is 12 bytes
    }

    #[test]
    fn test_get_file_metadata_with_nonexistent_file() {
        let result = get_file_metadata("/nonexistent/path.log".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read file metadata"));
    }

    #[tokio::test]
    async fn test_index_file_with_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");
        let mut file = File::create(&file_path).unwrap();

        // Write multiple RFC3164 entries to ensure format detection works
        for i in 0..100 {
            let day = (i % 28) + 1; // Valid days 1-28 for any month
            writeln!(file, "<134>Jan {} 00:00:00 firewall filterlog[123]: test entry {}", day, i).unwrap();
        }

        let result = index_file(file_path.to_string_lossy().to_string(), 1).await;

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.format, "RFC3164"); // Format should be detected
        assert_eq!(metadata.entry_count, 100); // All entries should be parsed
    }

    #[tokio::test]
    async fn test_index_file_with_nonexistent_path() {
        let result = index_file("/nonexistent/path.log".to_string(), 1).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
    }

    #[tokio::test]
    async fn test_index_file_with_invalid_compression_level() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");
        File::create(&file_path).unwrap();

        let result = index_file(file_path.to_string_lossy().to_string(), 99).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid compression level"));
    }

    #[tokio::test]
    async fn test_index_file_prevents_path_traversal() {
        // Test: Path traversal attacks should be rejected
        let result = index_file("../../../etc/passwd".to_string(), 1).await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        // canonicalize() will fail on nonexistent paths, which prevents traversal
        assert!(
            error_msg.contains("Invalid file path") || error_msg.contains("File not found"),
            "Expected path traversal to be blocked, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_index_file_with_directory_path() {
        // Test: Directories should be rejected
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();

        let result = index_file(dir_path, 1).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path is not a file"));
    }

    #[tokio::test]
    async fn test_index_file_with_relative_path() {
        // Test: Relative paths should be canonicalized and work if valid
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.log");
        let mut file = File::create(&file_path).unwrap();

        // Write multiple RFC3164 entries to ensure format detection works
        for i in 0..100 {
            let day = (i % 28) + 1; // Valid days 1-28 for any month
            writeln!(file, "<134>Jan {} 00:00:00 firewall filterlog[123]: test entry {}", day, i).unwrap();
        }

        // Get canonical absolute path
        let absolute_path = file_path.canonicalize().unwrap();
        let result = index_file(absolute_path.to_string_lossy().to_string(), 1).await;

        // Should succeed for valid absolute path
        assert!(result.is_ok());
    }
}
