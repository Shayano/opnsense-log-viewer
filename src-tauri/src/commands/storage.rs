use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, command};

use crate::storage::{calculate_file_hash, get_index_path, get_indexes_dir, load_index};
use crate::types::IndexMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub hash: String,
    pub source_file_path: String,
    pub source_file_exists: bool,
    pub created_at: DateTime<Utc>,
    pub file_size: u64,
    pub entry_count: u64,
    pub index_file_size: u64,
}

/// List all saved indexes
#[command]
pub fn list_all_indexes(app_handle: AppHandle) -> Result<Vec<IndexInfo>, String> {
    let indexes_dir = get_indexes_dir(&app_handle)
        .map_err(|e| format!("Failed to get indexes dir: {}", e))?;

    let mut indexes = Vec::new();

    for entry in fs::read_dir(&indexes_dir)
        .map_err(|e| format!("Failed to read indexes dir: {}", e))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("idx") {
            continue; // Skip non-.idx files
        }

        // Extract hash from filename (remove .idx extension)
        let hash = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Try to load index metadata
        match load_index(&path) {
            Ok(persisted_index) => {
                let source_exists =
                    fs::metadata(&persisted_index.source_metadata.file_path).is_ok();
                let index_file_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

                indexes.push(IndexInfo {
                    hash,
                    source_file_path: persisted_index.source_metadata.file_path,
                    source_file_exists: source_exists,
                    created_at: persisted_index.created_at,
                    file_size: persisted_index.source_metadata.file_size,
                    entry_count: persisted_index.source_metadata.entry_count,
                    index_file_size,
                });
            }
            Err(_) => {
                // Skip corrupted indexes
                continue;
            }
        }
    }

    // Sort by created_at descending (newest first)
    indexes.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(indexes)
}

/// Delete index by hash
#[command]
pub fn delete_index_by_hash(app_handle: AppHandle, hash: String) -> Result<(), String> {
    let indexes_dir = get_indexes_dir(&app_handle)
        .map_err(|e| format!("Failed to get indexes dir: {}", e))?;

    let index_path = indexes_dir.join(format!("{}.idx", hash));

    if !index_path.exists() {
        return Err(format!("Index not found: {}", hash));
    }

    fs::remove_file(&index_path).map_err(|e| format!("Failed to delete index: {}", e))?;

    Ok(())
}

/// Load an existing index file by source file path
/// Checks if index exists for file, verifies source file hash matches, loads index
#[command]
pub async fn load_index_file(
    app_handle: AppHandle,
    file_path: String,
) -> Result<IndexMetadata, String> {
    // 1. Calculate hash of source file
    let source_hash = calculate_file_hash(&file_path)
        .map_err(|e| format!("Failed to calculate file hash: {}", e))?;

    // 2. Check if index exists
    let index_path = get_index_path(&app_handle, &source_hash)
        .map_err(|e| format!("Failed to get index path: {}", e))?;

    if !index_path.exists() {
        return Err("No saved index found for this file".to_string());
    }

    // 3. Load index
    let persisted_index = load_index(&index_path)
        .map_err(|e| format!("Failed to load index: {}", e))?;

    // 4. Verify source file hash matches
    if persisted_index.source_file_hash != source_hash {
        return Err("File has been modified since indexing. Please re-index the file.".to_string());
    }

    // 5. Verify source file still exists
    if !std::path::Path::new(&file_path).exists() {
        return Err("Source file no longer exists at specified path".to_string());
    }

    // 6. Store in global state for queries (use shared HYBRID_INDEX)
    use super::query::HYBRID_INDEX;

    let new_mem = persisted_index.hybrid_index.memory_usage();
    {
        let mut guard = HYBRID_INDEX.lock().unwrap();
        let old_mem = guard.as_ref().map(|i| i.memory_usage()).unwrap_or(0);
        log::info!(
            "[MEM] load_index_file: replacing HYBRID_INDEX old≈{} bytes with loaded index≈{} bytes entry_count={}",
            old_mem,
            new_mem,
            persisted_index.source_metadata.entry_count
        );
        *guard = Some(persisted_index.hybrid_index.clone());
    }

    // 7. Return metadata
    let format_str = match persisted_index.source_metadata.log_format {
        crate::types::log_entry::LogFormat::RFC3164 => "RFC3164",
        crate::types::log_entry::LogFormat::RFC5424 => "RFC5424",
        crate::types::log_entry::LogFormat::CSV => "CSV",
        crate::types::log_entry::LogFormat::Unknown => "UNKNOWN",
    };

    Ok(IndexMetadata {
        source_file_hash: hex::encode(source_hash),
        entry_count: persisted_index.source_metadata.entry_count,
        format: format_str.to_string(),
        index_size_bytes: fs::metadata(&index_path)
            .map(|m| m.len())
            .unwrap_or(0),
        created_at: persisted_index.created_at.to_rfc3339(),
        parsing_stats: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_info_structure() {
        // Verify IndexInfo can be created and serialized
        let info = IndexInfo {
            hash: "abc123".to_string(),
            source_file_path: "/test/file.log".to_string(),
            source_file_exists: true,
            created_at: Utc::now(),
            file_size: 1024,
            entry_count: 100,
            index_file_size: 512,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("sourceFilePath")); // camelCase
        assert!(json.contains("createdAt"));
    }
}
