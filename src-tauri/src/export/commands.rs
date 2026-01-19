use crate::export::csv::CsvExporter;
use crate::export::json::JsonExporter;
use crate::export::types::{
    ExportFormat, ExportLogEntry, ExportMetadata, ExportProgress, ExportResult,
    ExportEstimate, ExportScope, VerificationResult,
};
use crate::export::utils::{check_disk_space, calculate_file_checksum, detect_incomplete_exports, cleanup_partial_export};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// Global cancellation flag for export operations
static EXPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

/// Estimate export file size and duration for warning dialog
#[tauri::command]
pub fn estimate_export(
    total_entries: usize,
    format: ExportFormat,
    _scope: ExportScope,
) -> Result<ExportEstimate, String> {
    // File size estimation (bytes per entry)
    let bytes_per_entry = match format {
        ExportFormat::Csv => 120.0,  // ~120 bytes per entry (compressed with typical data)
        ExportFormat::Json => 200.0, // ~200 bytes per entry (pretty-printed)
    };

    let estimated_file_size_bytes = total_entries as f64 * bytes_per_entry;
    let estimated_file_size_mb = estimated_file_size_bytes / 1_048_576.0;

    // Duration estimation (entries per second)
    let entries_per_second = match format {
        ExportFormat::Csv => 10_000.0,  // 10K entries/sec baseline for CSV
        ExportFormat::Json => 5_000.0,  // 5K entries/sec baseline for JSON
    };

    let estimated_duration_seconds = total_entries as f64 / entries_per_second;

    Ok(ExportEstimate {
        estimated_entries: total_entries,
        estimated_file_size_mb,
        estimated_duration_seconds,
    })
}

/// Export filtered results to CSV or JSON
///
/// Supports both in-memory export (for small datasets) and streaming export (for large datasets).
/// Uses streaming mode when total_entries > 10,000 to prevent memory overflow.
#[tauri::command]
pub async fn export_filtered_results(
    app: AppHandle,
    format: ExportFormat,
    entries: Vec<ExportLogEntry>,
    metadata: ExportMetadata,
    save_path: String,
    streaming: Option<bool>,
) -> Result<ExportResult, String> {
    // Reset cancellation flag
    EXPORT_CANCELLED.store(false, Ordering::SeqCst);

    let total_entries = entries.len();
    let save_path_buf = std::path::PathBuf::from(&save_path);

    // Validate inputs
    if save_path_buf.as_os_str().is_empty() {
        return Err("Invalid save path".to_string());
    }

    // Check disk space before export
    let estimate = estimate_export(total_entries, format, metadata.export_scope)
        .map_err(|e| format!("Failed to estimate export: {}", e))?;

    if let Err(disk_err) = check_disk_space(&save_path_buf, estimate.estimated_file_size_mb) {
        return Err(format!(
            "Insufficient disk space. Required: {:.2} MB, Available: {:.2} MB",
            disk_err.required_mb,
            disk_err.available_mb
        ));
    }

    // Decide whether to use streaming based on dataset size or explicit flag
    let use_streaming = streaming.unwrap_or(total_entries > 10_000);

    // Emit initial progress
    let _ = app.emit(
        "export-progress",
        ExportProgress {
            current: 0,
            total: total_entries,
            status: if use_streaming { "Starting streaming export..." } else { "Starting export..." }.to_string(),
            rows_per_second: 0.0,
            elapsed_seconds: 0.0,
        },
    );

    // Perform export in background
    let app_clone = app.clone();
    let cancel_flag = Arc::new(EXPORT_CANCELLED);
    let result = tokio::task::spawn_blocking(move || {
        if use_streaming {
            // Streaming mode - memory efficient for large datasets
            let start = Instant::now();
            let result = match format {
                ExportFormat::Csv => {
                    let exporter = CsvExporter::new(metadata);
                    exporter.export_streaming(
                        entries.into_iter(),
                        &save_path_buf,
                        total_entries,
                        |current, total| {
                            let elapsed = start.elapsed().as_secs_f64();
                            let rows_per_second = if elapsed > 0.0 { current as f64 / elapsed } else { 0.0 };
                            let _ = app_clone.emit(
                                "export-progress",
                                ExportProgress {
                                    current,
                                    total,
                                    status: format!("Exporting... {} of {}", current, total),
                                    rows_per_second,
                                    elapsed_seconds: elapsed,
                                },
                            );
                        },
                        cancel_flag.clone(),
                    )
                }
                ExportFormat::Json => {
                    let exporter = JsonExporter::new(metadata);
                    exporter.export_streaming(
                        entries.into_iter(),
                        &save_path_buf,
                        total_entries,
                        |current, total| {
                            let elapsed = start.elapsed().as_secs_f64();
                            let rows_per_second = if elapsed > 0.0 { current as f64 / elapsed } else { 0.0 };
                            let _ = app_clone.emit(
                                "export-progress",
                                ExportProgress {
                                    current,
                                    total,
                                    status: format!("Exporting... {} of {}", current, total),
                                    rows_per_second,
                                    elapsed_seconds: elapsed,
                                },
                            );
                        },
                        cancel_flag.clone(),
                    )
                }
            };
            result.map_err(|e| e.to_string())
        } else {
            // In-memory mode - faster for small datasets
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
        }
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

/// Verify export file integrity by checking embedded checksum
#[tauri::command]
pub fn verify_export_file(file_path: String) -> Result<VerificationResult, String> {
    let path = PathBuf::from(&file_path);

    // Verify file exists
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Get file size
    let file_size_bytes = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?
        .len();

    // Determine file type from extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| "File has no extension".to_string())?;

    match extension.to_lowercase().as_str() {
        "csv" => verify_csv_export(&path, file_size_bytes),
        "json" => verify_json_export(&path, file_size_bytes),
        _ => Err(format!("Unsupported file format: {}", extension)),
    }
}

/// Verify CSV export file
fn verify_csv_export(path: &PathBuf, file_size_bytes: u64) -> Result<VerificationResult, String> {
    // Read last 1 KB of file to find checksum
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let file_size = file.metadata().map_err(|e| format!("Failed to get metadata: {}", e))?.len();
    let read_size = std::cmp::min(file_size, 1024);

    let mut reader = BufReader::new(File::open(path).map_err(|e| format!("Failed to open file: {}", e))?);
    if file_size > read_size {
        reader
            .seek(SeekFrom::End(-(read_size as i64)))
            .map_err(|e| format!("Failed to seek: {}", e))?;
    }

    // Read lines and find checksum
    let mut expected_hash = String::new();
    let mut entries_count = None;

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        if line.starts_with("# Export SHA-256:") {
            expected_hash = line
                .trim_start_matches("# Export SHA-256:")
                .trim()
                .to_string();
        } else if line.starts_with("# Row Count Verification:") {
            // Parse entry count
            if let Some(count_str) = line.split_whitespace().nth(4) {
                entries_count = count_str.parse().ok();
            }
        }
    }

    if expected_hash.is_empty() {
        return Err("No checksum found in CSV file. File may be incomplete.".to_string());
    }

    // CRITICAL FIX: Calculate checksum on content ONLY (excluding checksum lines)
    // Read file and write temporary file without checksum lines, then calculate checksum
    let actual_hash = calculate_csv_content_checksum(path)
        .map_err(|e| format!("Failed to calculate checksum: {}", e))?;

    // Compare checksums
    let valid = expected_hash == actual_hash;

    Ok(VerificationResult {
        valid,
        expected_hash,
        actual_hash,
        file_size_bytes,
        entries_count,
    })
}

/// Calculate checksum of CSV content (excluding checksum comment lines)
/// This matches how the export process calculates the checksum
fn calculate_csv_content_checksum(path: &PathBuf) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Write;

    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);

    let mut hasher = Sha256::new();

    // Read line by line, hash everything EXCEPT checksum lines
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;

        // Skip checksum and row count verification lines
        if line.starts_with("# Export SHA-256:") || line.starts_with("# Row Count Verification:") {
            continue;
        }

        // Hash this line (including newline)
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }

    // Finalize hash
    let hash_bytes = hasher.finalize();
    let hash_hex = format!("{:x}", hash_bytes);

    Ok(hash_hex)
}

/// Verify JSON export file
fn verify_json_export(path: &PathBuf, file_size_bytes: u64) -> Result<VerificationResult, String> {
    // Read and parse JSON file
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut json: serde_json::Value = serde_json::from_reader(file)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Extract checksum from metadata
    let expected_hash = json
        .get("metadata")
        .and_then(|m| m.get("exportChecksum"))
        .and_then(|c| c.get("hash"))
        .and_then(|h| h.as_str())
        .ok_or_else(|| "No checksum found in JSON metadata. File may be incomplete.".to_string())?
        .to_string();

    // Extract entry count
    let entries_count = json
        .get("metadata")
        .and_then(|m| m.get("verification"))
        .and_then(|v| v.get("entriesWritten"))
        .and_then(|e| e.as_u64())
        .map(|c| c as usize);

    // CRITICAL FIX: Calculate checksum WITHOUT exportChecksum and verification fields
    // This matches how the export process calculates the checksum
    // Remove exportChecksum and verification from metadata before calculating
    if let Some(metadata) = json.get_mut("metadata") {
        if let Some(metadata_obj) = metadata.as_object_mut() {
            metadata_obj.remove("exportChecksum");
            metadata_obj.remove("verification");
        }
    }

    // Calculate checksum on JSON without exportChecksum/verification
    let actual_hash = {
        use sha2::{Digest, Sha256};
        let json_without_checksum = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        let mut hasher = Sha256::new();
        hasher.update(json_without_checksum.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    // Compare checksums
    let valid = expected_hash == actual_hash;

    Ok(VerificationResult {
        valid,
        expected_hash,
        actual_hash,
        file_size_bytes,
        entries_count,
    })
}

/// Detect incomplete exports on app startup
#[tauri::command]
pub fn detect_incomplete_export_files(export_dir: String) -> Result<Vec<String>, String> {
    let dir_path = PathBuf::from(export_dir);

    let incomplete = detect_incomplete_exports(dir_path)
        .map_err(|e| format!("Failed to detect incomplete exports: {}", e))?;

    // Convert PathBuf to String
    let incomplete_paths: Vec<String> = incomplete
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    Ok(incomplete_paths)
}

/// Clean up incomplete/partial export file
#[tauri::command]
pub fn cleanup_incomplete_export(file_path: String) -> Result<(), String> {
    let path = PathBuf::from(file_path);

    cleanup_partial_export(path)
        .map_err(|e| format!("Failed to cleanup partial export: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::types::{SourceFileInfo, FilterInfo};
    use chrono::Utc;
    use std::path::PathBuf;

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
        let path = PathBuf::from("");
        assert!(path.as_os_str().is_empty());

        let valid_path = PathBuf::from("/tmp/test.csv");
        assert!(!valid_path.as_os_str().is_empty());
    }

    #[test]
    fn test_estimate_export_csv() {
        let result = estimate_export(10000, ExportFormat::Csv, ExportScope::Filtered).unwrap();
        assert_eq!(result.estimated_entries, 10000);
        // ~120 bytes per entry -> ~1.14 MB for 10K entries
        assert!(result.estimated_file_size_mb > 1.0 && result.estimated_file_size_mb < 2.0);
        // 10K entries/sec -> ~1 second for 10K entries
        assert!(result.estimated_duration_seconds > 0.8 && result.estimated_duration_seconds < 1.2);
    }

    #[test]
    fn test_estimate_export_json() {
        let result = estimate_export(10000, ExportFormat::Json, ExportScope::FullDataset).unwrap();
        assert_eq!(result.estimated_entries, 10000);
        // ~200 bytes per entry -> ~1.91 MB for 10K entries
        assert!(result.estimated_file_size_mb > 1.8 && result.estimated_file_size_mb < 2.0);
        // 5K entries/sec -> ~2 seconds for 10K entries
        assert!(result.estimated_duration_seconds > 1.8 && result.estimated_duration_seconds < 2.2);
    }

    #[test]
    fn test_estimate_export_large_dataset() {
        // Test with 2M entries (full dataset scenario)
        let result = estimate_export(2_000_000, ExportFormat::Csv, ExportScope::FullDataset).unwrap();
        assert_eq!(result.estimated_entries, 2_000_000);
        // ~120 bytes per entry -> ~228 MB for 2M entries
        assert!(result.estimated_file_size_mb > 220.0 && result.estimated_file_size_mb < 240.0);
        // 10K entries/sec -> ~200 seconds for 2M entries
        assert!(result.estimated_duration_seconds > 190.0 && result.estimated_duration_seconds < 210.0);
    }

    #[test]
    fn test_estimate_export_empty() {
        let result = estimate_export(0, ExportFormat::Csv, ExportScope::Filtered).unwrap();
        assert_eq!(result.estimated_entries, 0);
        assert_eq!(result.estimated_file_size_mb, 0.0);
        assert_eq!(result.estimated_duration_seconds, 0.0);
    }
}
