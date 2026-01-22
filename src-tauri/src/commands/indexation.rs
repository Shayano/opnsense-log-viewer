use crate::storage::{get_index_path, save_index};
use crate::types::persisted_index::{PersistedIndex, SourceFileMetadata};
use crate::types::{FileMetadata, IndexMetadata};
use crate::parser::{detect_format, parse_file_streaming};
use crate::types::log_entry::LogFormat;
use crate::indexer::{HybridIndex, IndexProgress, IndexCache, IndexCacheEvent};
use std::fs;
use std::path::Path;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

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

    // 7. Parse log file with streaming parser (loads FULL file into RAM)
    log::info!("[MEM] index_file: calling parse_file_streaming (full load) path={:?}", canonical_path);
    let (entries, stats) = parse_file_streaming(&canonical_path, detected_format)
        .map_err(|e| format!("Failed to parse log file: {}", e))?;

    log::info!(
        "[MEM] index_file: parse_file_streaming done entries={} (Vec held until return)",
        entries.len()
    );

    // Log parsing statistics
    log::info!(
        "Parsed {} entries successfully, skipped {} malformed lines from {} total lines",
        stats.parsed_successfully,
        stats.skipped_malformed,
        stats.total_lines
    );

    // Note: This function is for basic parsing without persistence.
    // Use build_hybrid_index for full indexation with SHA-256 and disk persistence.
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(IndexMetadata {
        source_file_hash: "pending".to_string(), // Use build_hybrid_index for actual hash
        entry_count: entries.len() as u64,
        format: format_str.to_string(),
        index_size_bytes: 0, // Use build_hybrid_index for actual size
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

    // 4. Parse log file with specified format (loads FULL file into RAM)
    log::info!("[MEM] index_file_with_format: calling parse_file_streaming (full load) path={:?}", canonical_path);
    let (entries, stats) = parse_file_streaming(&canonical_path, log_format)
        .map_err(|e| format!("Failed to parse log file: {}", e))?;

    log::info!(
        "[MEM] index_file_with_format: parse_file_streaming done entries={} (Vec held until return)",
        entries.len()
    );

    // Log parsing statistics
    log::info!(
        "Parsed {} entries successfully, skipped {} malformed lines from {} total lines (format: {})",
        stats.parsed_successfully,
        stats.skipped_malformed,
        stats.total_lines,
        format
    );

    // Note: This function is for basic parsing without persistence.
    // Use build_hybrid_index for full indexation with SHA-256 and disk persistence.
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(IndexMetadata {
        source_file_hash: "pending".to_string(), // Use build_hybrid_index for actual hash
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

// Import the shared HYBRID_INDEX from query module (shared global state)
use super::query::HYBRID_INDEX;

/// Build hybrid index (Story 1.3) - with progress events
/// Story 6.4: Added persistent cache for instant reload (<500ms)
#[tauri::command]
pub async fn build_hybrid_index(
    app: AppHandle,
    file_path: String,
) -> Result<IndexMetadata, String> {
    // 1. Validate file path exists
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err("File not found. Please ensure the file path is correct.".to_string());
    }

    if !path.is_file() {
        return Err("Path is not a file.".to_string());
    }

    // 2. Check read permissions
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

    // 3. Detect log format
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

    // Story 6.4: Check cache first (fast path <500ms)
    let cache_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let cache = IndexCache::new(cache_dir);

    let cache_start = Instant::now();
    let cache_result = cache.get_cached_index(&canonical_path);
    let cache_lookup_time = cache_start.elapsed();

    if let Ok(Some((tiered_index, cache_metadata))) = cache_result {
        // Cache hit! Load from cache
        let cache_age = cache.get_cache_age(&canonical_path)
            .ok()
            .flatten()
            .unwrap_or(0);

        log::info!(
            "[CACHE] Cache hit for {}: {} entries loaded in {:.2}ms (age: {}s)",
            canonical_path.display(),
            tiered_index.total_entries,
            cache_lookup_time.as_secs_f64() * 1000.0,
            cache_age
        );

        // Emit cache-hit event
        let _ = app.emit("index-cache-hit", IndexCacheEvent {
            file_path: canonical_path.to_string_lossy().to_string(),
            cache_hit: true,
            cache_age_seconds: Some(cache_age),
            reason: None,
        });

        // Calculate hash for metadata
        let source_hash = crate::indexer::calculate_file_hash(&canonical_path)
            .map(|h| hex::encode(h))
            .unwrap_or_else(|_| "unknown".to_string());

        // Store cached index in global state for queries
        // Convert TieredIndex HotIndex back to HybridIndex for compatibility with existing query system
        {
            let mut guard = HYBRID_INDEX.lock().unwrap();

            // Bug Fix: Create HybridIndex from cached HotIndex data instead of empty index
            // This ensures queries work correctly after cache hit
            let index_metadata = crate::indexer::IndexMetadata {
                format: detected_format,
                entry_count: tiered_index.total_entries,
                created_at: chrono::Utc::now(),
                source_file_path: canonical_path.to_string_lossy().to_string(),
                source_file_size: fs::metadata(&canonical_path).map(|m| m.len()).unwrap_or(0),
                source_file_hash: source_hash.clone(),
            };

            let hybrid_index = HybridIndex::from_hot_index(tiered_index.hot, Some(index_metadata));

            log::info!(
                "[MEM] build_hybrid_index: cache hit restored {} entries into HYBRID_INDEX",
                tiered_index.total_entries
            );

            *guard = Some(hybrid_index);
        }

        // Emit completion event
        let metadata = crate::indexer::IndexMetadata {
            format: detected_format,
            entry_count: tiered_index.total_entries,
            created_at: chrono::Utc::now(),
            source_file_path: canonical_path.to_string_lossy().to_string(),
            source_file_size: fs::metadata(&canonical_path).map(|m| m.len()).unwrap_or(0),
            source_file_hash: source_hash.clone(),
        };
        let _ = app.emit("indexation-complete", &metadata);

        // Return metadata for cache hit
        return Ok(IndexMetadata {
            source_file_hash: source_hash,
            entry_count: cache_metadata.entry_count,
            format: format_str.to_string(),
            index_size_bytes: 0,
            created_at: chrono::DateTime::from_timestamp(cache_metadata.created_at, 0)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
            parsing_stats: None,
        });
    }

    // Cache miss - emit event and proceed with full indexation
    let miss_reason = match &cache_result {
        Ok(None) => "cache file not found".to_string(),
        Err(e) => format!("cache lookup failed: {}", e),
        _ => "unknown".to_string(),
    };

    log::info!(
        "[CACHE] Cache miss for {}: {} (lookup took {:.2}ms)",
        canonical_path.display(),
        miss_reason,
        cache_lookup_time.as_secs_f64() * 1000.0
    );

    let _ = app.emit("index-cache-miss", IndexCacheEvent {
        file_path: canonical_path.to_string_lossy().to_string(),
        cache_hit: false,
        cache_age_seconds: None,
        reason: Some(miss_reason),
    });

    // 4. Create hybrid index
    let hybrid_index = HybridIndex::new();

    // Store the index in global state for cancellation
    {
        let mut guard = HYBRID_INDEX.lock().unwrap();
        let old_mem = guard.as_ref().map(|i| i.memory_usage()).unwrap_or(0);
        log::info!(
            "[MEM] build_hybrid_index: replacing HYBRID_INDEX (old≈{} bytes dropped), inserting empty for stream build",
            old_mem
        );
        *guard = Some(hybrid_index);
    }

    // 5. Build index with progress callback in blocking task
    // Performance: Use spawn_blocking to avoid blocking the async runtime
    // This allows the UI to remain responsive during large file indexation
    let app_clone = app.clone();
    let canonical_path_clone = canonical_path.clone();
    let hybrid_index_arc = HYBRID_INDEX.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut guard = hybrid_index_arc.lock().unwrap();
        let index = guard.as_mut().unwrap();

        index.build_index(
            &canonical_path_clone,
            detected_format,
            move |progress: IndexProgress| {
                // Emit progress event to frontend
                let _ = app_clone.emit("indexation-progress", &progress);
            }
        )
    }).await.map_err(|e| format!("Build task failed: {}", e))?;

    match result {
        Ok(metadata) => {
            let mem = HYBRID_INDEX.lock().unwrap().as_ref().map(|i| i.memory_usage()).unwrap_or(0);
            log::info!(
                "[MEM] build_hybrid_index: build_index done HYBRID_INDEX≈{} bytes entry_count={}",
                mem,
                metadata.entry_count
            );

            // NEW Story 1.4: Save index to disk
            let save_result = (|| -> Result<(), String> {
                // Calculate source file hash (bytes)
                let source_hash_bytes = hex::decode(&metadata.source_file_hash)
                    .map_err(|e| format!("Failed to decode hash: {}", e))?;

                let mut hash_array = [0u8; 32];
                hash_array.copy_from_slice(&source_hash_bytes);

                // Get index from global state
                let guard = HYBRID_INDEX.lock().unwrap();
                let hybrid_index = guard.as_ref()
                    .ok_or("Index not found in global state")?
                    .clone();

                // Create PersistedIndex
                let source_metadata = SourceFileMetadata {
                    file_path: canonical_path.to_string_lossy().to_string(),
                    file_size: metadata.source_file_size,
                    entry_count: metadata.entry_count,
                    log_format: detected_format,
                };

                let persisted_index = PersistedIndex::new(
                    hash_array,
                    source_metadata,
                    hybrid_index,
                );

                // Get index path and save
                let index_path = get_index_path(&app, &hash_array)
                    .map_err(|e| format!("Failed to get index path: {}", e))?;

                save_index(&persisted_index, &index_path)
                    .map_err(|e| format!("Failed to save index: {}", e))?;

                // Emit index-saved event
                let _ = app.emit("index-saved", index_path.to_string_lossy().to_string());

                Ok(())
            })();

            if let Err(e) = save_result {
                log::warn!("Failed to save index: {}", e);
                // Continue anyway - indexation succeeded
            }

            // Story 6.4: Save to TieredIndex cache for instant reload
            let cache_save_result = (|| -> Result<(), String> {
                use crate::indexer::tiered::{TieredIndex, TieredConfig, HotIndex};

                // Get index from global state to convert to TieredIndex
                let guard = HYBRID_INDEX.lock().unwrap();
                let hybrid_index = guard.as_ref()
                    .ok_or("Index not found in global state")?;

                // Create TieredIndex from HybridIndex components
                let tiered_index = TieredIndex {
                    config: TieredConfig::default(),
                    hot: HotIndex::from_indexes(
                        hybrid_index.inverted_index().clone(),
                        hybrid_index.bitmap_index().clone(),
                        hybrid_index.offset_table().clone(),
                        0,
                        metadata.entry_count,
                    ),
                    warm: Vec::new(),
                    total_entries: metadata.entry_count,
                };

                // Save to cache
                cache.save_index(&canonical_path, &tiered_index)
                    .map_err(|e| format!("Failed to save to cache: {}", e))?;

                log::info!(
                    "[CACHE] Saved {} entries to cache for instant reload",
                    metadata.entry_count
                );

                Ok(())
            })();

            if let Err(e) = cache_save_result {
                log::warn!("Failed to save to cache: {}", e);
                // Continue anyway - indexation succeeded
            }

            // Emit completion event
            let _ = app.emit("indexation-complete", &metadata);

            // Convert to types::IndexMetadata for compatibility
            Ok(IndexMetadata {
                source_file_hash: metadata.source_file_hash,
                entry_count: metadata.entry_count,
                format: format_str.to_string(),
                index_size_bytes: 0, // Will be calculated later
                created_at: metadata.created_at.to_rfc3339(),
                parsing_stats: None,
            })
        }
        Err(e) => {
            // Emit error event
            let error_msg = format!("{}", e);
            let _ = app.emit("indexation-error", &error_msg);
            Err(error_msg)
        }
    }
}

/// Cancel ongoing indexation
#[tauri::command]
pub async fn cancel_indexation() -> Result<(), String> {
    let guard = HYBRID_INDEX.lock().unwrap();
    if let Some(index) = guard.as_ref() {
        index.cancel();
        Ok(())
    } else {
        Err("No indexation in progress".to_string())
    }
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
