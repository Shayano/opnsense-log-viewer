//! SQLite-Based Indexation Commands
//!
//! Story 6.3: Provides Tauri async commands for SQLite-based log indexation
//! with real-time progress events and non-blocking UI.
//!
//! ## Architecture
//!
//! This module bridges the parallel parsing pipeline (Story 6.2) with the Tauri
//! frontend via async commands and progress events:
//!
//! ```text
//! Frontend <--> Tauri Command <--> Pipeline <--> SQLite
//!    ^                               |
//!    |                               v
//!    +---- Progress Events <-- Progress Emitter (1 Hz)
//! ```

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::indexer::sqlite::{
    get_or_create_database, build_sqlite_index as pipeline_build_index,
    PipelineProgress, IndexStats,
};
use crate::indexer::progress::IndexProgress;
use crate::parser::detect_format;
use crate::types::log_entry::LogFormat;
use crate::storage::calculate_file_hash_quick;

use super::progress_emitter::ProgressEmitter;
use super::sqlite_query::set_sqlite_pool;

/// Metadata returned after SQLite index building
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteIndexMetadata {
    /// Total entries successfully indexed
    pub entry_count: u64,
    /// Total time elapsed in milliseconds
    pub elapsed_ms: u64,
    /// Processing speed in entries per second
    pub entries_per_second: u64,
    /// Bytes processed from source file
    pub bytes_processed: u64,
    /// Bytes per second throughput
    pub bytes_per_second: u64,
    /// File hash for cache identification
    pub file_hash: String,
    /// Detected log format
    pub format: String,
}

impl From<IndexStats> for SqliteIndexMetadata {
    fn from(stats: IndexStats) -> Self {
        Self {
            entry_count: stats.entry_count,
            elapsed_ms: stats.elapsed_ms,
            entries_per_second: stats.entries_per_second,
            bytes_processed: stats.bytes_processed,
            bytes_per_second: stats.bytes_per_second,
            file_hash: String::new(), // Set separately
            format: String::new(),    // Set separately
        }
    }
}

/// Build a SQLite index for a log file with real-time progress events
///
/// This command is non-blocking and emits progress events at 1 Hz.
/// The UI thread is never blocked by Rust computation.
///
/// # Arguments
/// * `app` - Tauri app handle for event emission
/// * `file_path` - Path to the log file to index
///
/// # Returns
/// * `Ok(SqliteIndexMetadata)` - Metadata about the indexed file
/// * `Err(String)` - Error message if indexation fails
///
/// # Events Emitted
/// * `indexation-progress` - Progress updates every 1 second
/// * `indexation-complete` - Final completion event
/// * `indexation-error` - Error event if indexation fails
#[tauri::command]
pub async fn build_sqlite_index(
    app: AppHandle,
    file_path: String,
) -> Result<SqliteIndexMetadata, String> {
    // 1. Validate file path (reuse validation from indexation.rs)
    let path = validate_file_path(&file_path)?;

    // 2. Get file size for progress tracking
    let file_size = fs::metadata(&path)
        .map_err(|e| format!("Failed to get file size: {}", e))?
        .len();

    // 3. Detect log format
    let format = detect_format(&path)
        .map_err(|e| format!("Failed to detect format: {}", e))?;

    let format_str = match format {
        LogFormat::RFC3164 => "RFC3164",
        LogFormat::RFC5424 => "RFC5424",
        LogFormat::CSV => "CSV",
        LogFormat::Unknown => {
            return Err("Unable to auto-detect log format. Please select format manually.".to_string());
        }
    };

    // 4. Calculate file hash (Story 6.5: migrated to storage module)
    let file_hash = calculate_file_hash_quick(&path)
        .map_err(|e| format!("Failed to calculate file hash: {}", e))?;
    let file_hash_hex = hex::encode(&file_hash);

    // 5. Get or create SQLite database (Story 6.1)
    let pool = get_or_create_database(&app, &path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // 6. Create shared progress tracker
    let progress = Arc::new(PipelineProgress::new());

    // 7. Emit "Starting import..." immediately (AC1: within 100ms)
    let initial_progress = IndexProgress::new(0, file_size, 0.0);
    let _ = app.emit("indexation-progress", &initial_progress);

    log::info!(
        "[SQLITE CMD] Starting SQLite index build for {} ({} bytes, {} format)",
        path.display(),
        file_size,
        format_str
    );

    // 8. Spawn progress emitter in dedicated thread (1 Hz)
    let emitter_progress = Arc::clone(&progress);
    let emitter_app = app.clone();
    let emitter_handle = thread::spawn(move || {
        ProgressEmitter::new(emitter_app, emitter_progress, file_size).run();
    });

    // 9. Run pipeline in blocking task (keeps async runtime free)
    // Note: pipeline_build_index creates its own internal PipelineProgress
    // The emitter's progress is used for UI updates via the shared Arc
    let pipeline_pool = pool;
    let pipeline_path = path.clone();
    let pipeline_hash = file_hash_hex.clone();
    let pipeline_format = format;

    let result = tokio::task::spawn_blocking(move || {
        pipeline_build_index(
            &pipeline_pool,
            &pipeline_path,
            pipeline_format,
            &pipeline_hash,
            file_size,
        )
    })
    .await
    .map_err(|e| format!("Pipeline task panicked: {}", e))?;

    // 10. Wait for emitter to finish (it checks is_complete)
    // The emitter will exit naturally when it detects completion
    let _ = emitter_handle.join();

    // 11. Handle result
    match result {
        Ok(stats) => {
            log::info!(
                "[SQLITE CMD] Index build complete: {} entries in {}ms ({} entries/sec)",
                stats.entry_count,
                stats.elapsed_ms,
                stats.entries_per_second
            );

            // Build metadata
            let mut metadata: SqliteIndexMetadata = stats.into();
            metadata.file_hash = file_hash_hex.clone();
            metadata.format = format_str.to_string();

            // Story 6.4: Set the SQLite pool for query commands
            // Re-acquire pool from cache since original was moved to pipeline
            let query_pool = get_or_create_database(&app, &path)
                .map_err(|e| format!("Failed to get SQLite pool for queries: {}", e))?;
            set_sqlite_pool(Arc::new(query_pool), file_hash_hex)
                .map_err(|e| format!("Failed to set SQLite pool: {}", e))?;

            // Emit completion event with final statistics (AC6)
            let final_progress = IndexProgress::with_batch_info(
                file_size,
                file_size,
                metadata.elapsed_ms as f64 / 1000.0,
                metadata.entry_count,
                metadata.entry_count,
                true,
                1,
                1,
            );
            let _ = app.emit("indexation-complete", &final_progress);

            Ok(metadata)
        }
        Err(e) => {
            let error_msg = format!("Pipeline error: {}", e);
            log::error!("[SQLITE CMD] Index build failed: {}", error_msg);

            // Emit error event
            let _ = app.emit("indexation-error", &error_msg);

            Err(error_msg)
        }
    }
}

/// Validate file path for indexation
///
/// Reuses validation logic from commands/indexation.rs:
/// - File exists check
/// - Path canonicalization
/// - Read permission check
fn validate_file_path(file_path: &str) -> Result<std::path::PathBuf, String> {
    let path = Path::new(file_path);

    // Check file exists
    if !path.exists() {
        return Err("File not found. Please ensure the file path is correct.".to_string());
    }

    if !path.is_file() {
        return Err("Path is not a file.".to_string());
    }

    // Canonicalize path to prevent path traversal
    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Invalid file path: {}. Please check the path and try again.", e))?;

    // Ensure path is absolute
    if !canonical_path.is_absolute() {
        return Err("Invalid file path: path must be absolute.".to_string());
    }

    // Verify read permissions
    fs::File::open(&canonical_path)
        .map_err(|e| format!("Failed to open file: {}. Please check file permissions and try again.", e))?;

    Ok(canonical_path)
}

/// Cancel ongoing SQLite indexation
///
/// Note: Currently not implemented - cancellation requires pipeline support.
/// The progress emitter will stop when the pipeline completes or fails.
#[tauri::command]
pub async fn cancel_sqlite_indexation() -> Result<(), String> {
    // TODO: Story 6.3 Task 6.5 - Implement cancellation via atomic flag in PipelineProgress
    Err("SQLite indexation cancellation not yet implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_path_nonexistent() {
        let result = validate_file_path("/nonexistent/path.log");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_validate_file_path_directory() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        let result = validate_file_path(&temp.path().to_string_lossy());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a file"));
    }

    #[test]
    fn test_validate_file_path_valid() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "test content").unwrap();

        let result = validate_file_path(&file.path().to_string_lossy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_sqlite_index_metadata_from_index_stats() {
        let stats = IndexStats {
            entry_count: 1000,
            error_count: 5,
            elapsed_ms: 2000,
            entries_per_second: 500,
            bytes_processed: 100000,
            bytes_per_second: 50000,
        };

        let metadata: SqliteIndexMetadata = stats.into();

        assert_eq!(metadata.entry_count, 1000);
        assert_eq!(metadata.elapsed_ms, 2000);
        assert_eq!(metadata.entries_per_second, 500);
        assert_eq!(metadata.bytes_processed, 100000);
        assert_eq!(metadata.bytes_per_second, 50000);
    }
}
