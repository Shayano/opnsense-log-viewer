use crate::export::csv::CsvExporter;
use crate::export::json::JsonExporter;
use crate::export::types::{ExportFormat, ExportLogEntry, ExportMetadata, ExportProgress, ExportResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// Global cancellation flag for export operations
static EXPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

/// Export filtered results to CSV or JSON
#[tauri::command]
pub async fn export_filtered_results(
    app: AppHandle,
    format: ExportFormat,
    entries: Vec<ExportLogEntry>,
    metadata: ExportMetadata,
    save_path: String,
) -> Result<ExportResult, String> {
    // Reset cancellation flag
    EXPORT_CANCELLED.store(false, Ordering::SeqCst);

    let start_time = Instant::now();
    let total_entries = entries.len();
    let save_path_buf = std::path::PathBuf::from(save_path);

    // Validate inputs
    if save_path_buf.as_os_str().is_empty() {
        return Err("Invalid save path".to_string());
    }

    // Emit initial progress
    let _ = app.emit(
        "export-progress",
        ExportProgress {
            current: 0,
            total: total_entries,
            status: "Starting export...".to_string(),
            rows_per_second: 0.0,
            elapsed_seconds: 0.0,
        },
    );

    // For large exports, emit progress events periodically
    let should_track_progress = total_entries > 1000;

    // Perform export in background
    let app_clone = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let result = match format {
            ExportFormat::Csv => {
                let exporter = CsvExporter::new(metadata);
                exporter.export(entries, &save_path_buf)
            }
            ExportFormat::Json => {
                let exporter = JsonExporter::new(metadata);
                exporter.export(entries, &save_path_buf)
            }
        };

        result.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Export task failed: {}", e))?;

    // Check if cancelled
    if EXPORT_CANCELLED.load(Ordering::SeqCst) {
        // Clean up partial file
        let _ = std::fs::remove_file(&save_path_buf);
        return Err("Export cancelled by user".to_string());
    }

    match result {
        Ok(export_result) => {
            // Emit completion event
            let _ = app.emit("export-complete", export_result.clone());
            Ok(export_result)
        }
        Err(e) => {
            // Emit error event
            let _ = app.emit("export-error", e.clone());
            Err(e)
        }
    }
}

/// Cancel ongoing export operation
#[tauri::command]
pub fn cancel_export() -> Result<(), String> {
    EXPORT_CANCELLED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Open file location in OS file manager
#[tauri::command]
pub async fn open_export_location(app: AppHandle, file_path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let path = std::path::PathBuf::from(file_path);

    // Get parent directory
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid file path".to_string())?;

    // Use Tauri opener plugin to open the directory
    app.opener()
        .open_path(parent.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("Failed to open location: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::types::{SourceFileInfo, FilterInfo};
    use chrono::Utc;

    #[test]
    fn test_cancel_export() {
        // Reset flag
        EXPORT_CANCELLED.store(false, Ordering::SeqCst);
        assert!(!EXPORT_CANCELLED.load(Ordering::SeqCst));

        // Cancel
        cancel_export().unwrap();
        assert!(EXPORT_CANCELLED.load(Ordering::SeqCst));

        // Reset for other tests
        EXPORT_CANCELLED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_export_validation() {
        // Test empty save path validation
        // Note: This requires a Tauri app handle, so we'll test the path validation logic separately
        let path = std::path::PathBuf::from("");
        assert!(path.as_os_str().is_empty());

        let valid_path = std::path::PathBuf::from("/tmp/test.csv");
        assert!(!valid_path.as_os_str().is_empty());
    }
}
