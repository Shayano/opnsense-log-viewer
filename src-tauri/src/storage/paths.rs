use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Failed to resolve application data directory")]
    AppDataDirNotFound,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Get the indexes directory path (OS-specific)
/// - Windows: %APPDATA%\opnsense-log-viewer\indexes\
/// - macOS: ~/Library/Application Support/opnsense-log-viewer/indexes/
/// - Linux: ~/.local/share/opnsense-log-viewer/indexes/
pub fn get_indexes_dir(app_handle: &AppHandle) -> Result<PathBuf, PathError> {
    // Use Tauri's app_data_dir() helper for cross-platform compatibility
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| PathError::AppDataDirNotFound)?;

    let indexes_dir = app_data_dir.join("indexes");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&indexes_dir)?;

    Ok(indexes_dir)
}

/// Get full path for an index file based on source file hash
pub fn get_index_path(
    app_handle: &AppHandle,
    source_hash: &[u8; 32],
) -> Result<PathBuf, PathError> {
    let indexes_dir = get_indexes_dir(app_handle)?;
    let filename = format!("{}.idx", hex::encode(source_hash));
    Ok(indexes_dir.join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_filename_format() {
        // Test that hash converts to proper filename
        let hash = [0xAB; 32];
        let hex_str = hex::encode(hash);
        assert_eq!(hex_str.len(), 64); // 32 bytes = 64 hex chars

        let filename = format!("{}.idx", hex_str);
        assert!(filename.ends_with(".idx"));
        assert_eq!(filename.len(), 68); // 64 + ".idx"
    }

    // Note: Full integration tests with Tauri AppHandle require Tauri test context
    // These will be covered in integration tests
}
