use crate::types::{FileMetadata, IndexMetadata};
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
    match fs::metadata(&path) {
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

    // TODO Story 1.3: Call indexer::index_file(canonical_path, compression_level)
    // For now, return mock metadata to enable development of frontend components
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(IndexMetadata {
        source_file_hash: "pending".to_string(), // Will be calculated in Story 1.3
        entry_count: 0,
        format: "UNKNOWN".to_string(),
        index_size_bytes: 0,
        created_at,
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
        file.write_all(b"<134>Jan 1 00:00:00 firewall filterlog: test\n").unwrap();

        let result = index_file(file_path.to_string_lossy().to_string(), 1).await;

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.format, "UNKNOWN"); // Will be detected in Story 1.2/1.3
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
}
