//! SQLite Query Commands
//!
//! Story 6.4: Provides Tauri commands for executing queries against SQLite database.
//!
//! ## Architecture
//!
//! This module bridges the SqliteQueryExecutor with the Tauri frontend:
//!
//! ```text
//! Frontend <--> Tauri Command <--> SqliteQueryExecutor <--> SQLite Pool
//! ```
//!
//! ## Global State
//!
//! The SQLite connection pool is stored in a global state that is set during
//! indexation (Story 6.3). Queries use read connections from this pool.

use std::sync::{Arc, Mutex};

use log::info;
use serde::{Deserialize, Serialize};

use crate::indexer::sqlite::SqliteConnectionPool;
use crate::query::{
    FilterCondition, QueryError, QueryResult, SqliteLogEntry, SqliteQueryExecutor,
    MAX_ENTRY_IDS_IN_RESULT,
};

lazy_static::lazy_static! {
    /// Global SQLite connection pool for queries
    ///
    /// Set by `set_sqlite_pool` after indexation completes.
    /// Used by query commands for read operations.
    pub static ref SQLITE_POOL: Arc<Mutex<Option<Arc<SqliteConnectionPool>>>> =
        Arc::new(Mutex::new(None));

    /// File hash of the currently loaded SQLite database
    ///
    /// Used to verify queries target the correct database.
    pub static ref SQLITE_FILE_HASH: Arc<Mutex<Option<String>>> =
        Arc::new(Mutex::new(None));
}

/// Set the global SQLite pool for queries
///
/// Called by `build_sqlite_index` after successful indexation.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `file_hash` - SHA-256 hash of the source log file
pub fn set_sqlite_pool(pool: Arc<SqliteConnectionPool>, file_hash: String) -> Result<(), String> {
    {
        let mut pool_guard = SQLITE_POOL
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_POOL: {}", e))?;
        *pool_guard = Some(pool);
    }

    {
        let mut hash_guard = SQLITE_FILE_HASH
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_FILE_HASH: {}", e))?;
        *hash_guard = Some(file_hash.clone());
    }

    log::info!("[SQLITE QUERY] Pool set for file hash: {}", file_hash);
    Ok(())
}

/// Clear the global SQLite pool
#[allow(dead_code)]
pub fn clear_sqlite_pool() -> Result<(), String> {
    {
        let mut pool_guard = SQLITE_POOL
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_POOL: {}", e))?;
        *pool_guard = None;
    }

    {
        let mut hash_guard = SQLITE_FILE_HASH
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_FILE_HASH: {}", e))?;
        *hash_guard = None;
    }

    log::info!("[SQLITE QUERY] Pool cleared");
    Ok(())
}

/// Query request from frontend
///
/// Matches the existing QueryRequest but uses index_hash for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteQueryRequest {
    pub filters: Vec<FilterCondition>,
    pub index_hash: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

/// Execute a query against the SQLite database
///
/// This command processes filter conditions with boolean logic (AND/OR/NOT)
/// and returns matching entry IDs with total count for pagination.
///
/// # Arguments
/// * `request` - Query request containing filters and index hash
///
/// # Returns
/// * `Ok(QueryResult)` - Query results with entry IDs and execution time
/// * `Err(String)` - Error message if query execution fails
#[tauri::command]
pub async fn execute_sqlite_query(request: SqliteQueryRequest) -> Result<QueryResult, String> {
    info!(
        "Executing SQLite query with {} filters",
        request.filters.len()
    );

    // Get pool from global state
    let pool = {
        let pool_guard = SQLITE_POOL
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_POOL: {}", e))?;
        pool_guard
            .clone()
            .ok_or_else(|| "SQLite pool not initialized. Import a file first.".to_string())?
    };

    // Verify index hash matches (if provided)
    if !request.index_hash.is_empty() {
        let current_hash = {
            let hash_guard = SQLITE_FILE_HASH
                .lock()
                .map_err(|e| format!("Failed to lock SQLITE_FILE_HASH: {}", e))?;
            hash_guard.clone()
        };

        if let Some(ref expected_hash) = current_hash {
            if expected_hash != &request.index_hash {
                return Err(format!(
                    "Index hash mismatch. Expected '{}' but loaded database has '{}'. Please reload the log file.",
                    request.index_hash, expected_hash
                ));
            }
        }
    }

    // Create executor and run query
    let executor = SqliteQueryExecutor::new(pool);

    let limit = request.limit.unwrap_or(MAX_ENTRY_IDS_IN_RESULT);
    let offset = request.offset.unwrap_or(0);

    let result = executor
        .query(&request.filters, limit, offset)
        .map_err(query_error_to_string)?;

    info!(
        "[SQLITE QUERY] completed: {} matches, {} IDs returned, {} total ({}ms)",
        result.matched_count,
        result.entry_ids.len(),
        result.total_count,
        result.execution_time_ms
    );

    Ok(result)
}

/// DTO for log entry from SQLite (matches frontend LogEntry type)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteLogEntryDto {
    pub id: String,
    pub timestamp: String,
    pub interface: String,
    pub source_ip: String,
    pub source_port: u16,
    pub destination_ip: String,
    pub destination_port: u16,
    pub protocol: String,
    pub action: String,
    pub rule_label: String,
}

impl From<SqliteLogEntry> for SqliteLogEntryDto {
    fn from(entry: SqliteLogEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            timestamp: entry.timestamp,
            interface: entry.interface.unwrap_or_default(),
            source_ip: entry.source_ip.unwrap_or_default(),
            source_port: entry.source_port.unwrap_or(0),
            destination_ip: entry.dest_ip.unwrap_or_default(),
            destination_port: entry.dest_port.unwrap_or(0),
            protocol: entry
                .protocol
                .unwrap_or_else(|| "unknown".to_string())
                .to_lowercase(),
            action: entry.action.to_lowercase(),
            rule_label: entry.rule_id.unwrap_or_default(),
        }
    }
}

/// Fetch full log entries by IDs from SQLite database
///
/// Retrieves entries directly from SQLite instead of re-parsing the source file.
/// Much faster than the hybrid index approach for large files.
///
/// # Arguments
/// * `entry_ids` - List of entry IDs to fetch
///
/// # Returns
/// * `Ok(Vec<SqliteLogEntryDto>)` - Entries found
/// * `Err(String)` - Error message if fetch fails
#[tauri::command]
pub async fn get_sqlite_entries_by_ids(
    entry_ids: Vec<u64>,
) -> Result<Vec<SqliteLogEntryDto>, String> {
    let requested = entry_ids.len();

    log::info!(
        "[MEM] get_sqlite_entries_by_ids: request_ids={}",
        requested
    );

    if entry_ids.is_empty() {
        return Ok(vec![]);
    }

    // Get pool from global state
    let pool = {
        let pool_guard = SQLITE_POOL
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_POOL: {}", e))?;
        pool_guard
            .clone()
            .ok_or_else(|| "SQLite pool not initialized. Import a file first.".to_string())?
    };

    // Cap to MAX_ENTRY_IDS_IN_RESULT
    let capped_ids: Vec<u64> = entry_ids
        .into_iter()
        .take(MAX_ENTRY_IDS_IN_RESULT)
        .collect();

    let executor = SqliteQueryExecutor::new(pool);

    let entries = executor
        .get_entries_by_ids(&capped_ids)
        .map_err(query_error_to_string)?;

    let dtos: Vec<SqliteLogEntryDto> = entries.into_iter().map(|e| e.into()).collect();

    info!("Fetched {} SQLite entries for display", dtos.len());
    log::info!(
        "[MEM] get_sqlite_entries_by_ids: done result_len={}",
        dtos.len()
    );

    Ok(dtos)
}

/// Get total entry count from SQLite database
///
/// # Returns
/// * `Ok(u64)` - Total entry count
/// * `Err(String)` - Error message if count fails
#[tauri::command]
pub async fn get_sqlite_entry_count() -> Result<u64, String> {
    let pool = {
        let pool_guard = SQLITE_POOL
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_POOL: {}", e))?;
        pool_guard
            .clone()
            .ok_or_else(|| "SQLite pool not initialized. Import a file first.".to_string())?
    };

    let executor = SqliteQueryExecutor::new(pool);
    executor
        .total_entries()
        .map_err(query_error_to_string)
}

/// Get current SQLite database info
///
/// Returns information about the loaded SQLite database for debugging.
#[tauri::command]
pub async fn get_sqlite_database_info() -> Result<SqliteDatabaseInfo, String> {
    let (pool, hash) = {
        let pool_guard = SQLITE_POOL
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_POOL: {}", e))?;

        let hash_guard = SQLITE_FILE_HASH
            .lock()
            .map_err(|e| format!("Failed to lock SQLITE_FILE_HASH: {}", e))?;

        (pool_guard.clone(), hash_guard.clone())
    };

    match pool {
        Some(p) => {
            let entry_count = p.get_entry_count().unwrap_or(0);
            let db_path = p.db_path().to_string_lossy().to_string();

            Ok(SqliteDatabaseInfo {
                is_loaded: true,
                file_hash: hash,
                entry_count: Some(entry_count),
                db_path: Some(db_path),
            })
        }
        None => Ok(SqliteDatabaseInfo {
            is_loaded: false,
            file_hash: None,
            entry_count: None,
            db_path: None,
        }),
    }
}

/// SQLite database info DTO
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteDatabaseInfo {
    pub is_loaded: bool,
    pub file_hash: Option<String>,
    pub entry_count: Option<u64>,
    pub db_path: Option<String>,
}

/// Convert QueryError to user-friendly string
fn query_error_to_string(e: QueryError) -> String {
    match e {
        QueryError::InvalidRegex(msg) => format!("Invalid regex pattern: {}", msg),
        QueryError::IndexNotFound => "Index not found or hash mismatch".to_string(),
        QueryError::NoFilters => "No filters provided".to_string(),
        QueryError::ExecutionError(msg) => format!("Query execution failed: {}", msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::types::{FilterField, FilterOperator, FilterValue};

    #[tokio::test]
    async fn test_execute_sqlite_query_no_pool() {
        // Clear any existing pool
        {
            let mut guard = SQLITE_POOL.lock().unwrap();
            *guard = None;
        }

        let request = SqliteQueryRequest {
            filters: vec![FilterCondition {
                field: FilterField::Action,
                operator: FilterOperator::Equals,
                value: FilterValue::String("block".to_string()),
                logic: None,
            }],
            index_hash: "test".to_string(),
            limit: None,
            offset: None,
        };

        let result = execute_sqlite_query(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not initialized"));
    }

    #[tokio::test]
    async fn test_get_sqlite_entries_empty() {
        // Should return empty vec for empty input
        let result = get_sqlite_entries_by_ids(vec![]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_sqlite_database_info_no_pool() {
        // Clear any existing pool
        {
            let mut guard = SQLITE_POOL.lock().unwrap();
            *guard = None;
        }

        let result = get_sqlite_database_info().await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(!info.is_loaded);
        assert!(info.file_hash.is_none());
    }

    #[test]
    fn test_sqlite_log_entry_dto_conversion() {
        let entry = SqliteLogEntry {
            id: 1,
            timestamp: "2026-01-22T10:00:00Z".to_string(),
            source_ip: Some("192.168.1.1".to_string()),
            source_port: Some(12345),
            dest_ip: Some("10.0.0.1".to_string()),
            dest_port: Some(443),
            action: "PASS".to_string(),
            protocol: Some("TCP".to_string()),
            interface: Some("vtnet0".to_string()),
            rule_id: Some("rule_1".to_string()),
            raw_line: None,
        };

        let dto: SqliteLogEntryDto = entry.into();

        assert_eq!(dto.id, "1");
        assert_eq!(dto.source_ip, "192.168.1.1");
        assert_eq!(dto.action, "pass"); // lowercase
        assert_eq!(dto.protocol, "tcp"); // lowercase
    }

    #[test]
    fn test_sqlite_log_entry_dto_defaults() {
        let entry = SqliteLogEntry {
            id: 1,
            timestamp: "2026-01-22T10:00:00Z".to_string(),
            source_ip: None,
            source_port: None,
            dest_ip: None,
            dest_port: None,
            action: "pass".to_string(),
            protocol: None,
            interface: None,
            rule_id: None,
            raw_line: None,
        };

        let dto: SqliteLogEntryDto = entry.into();

        assert_eq!(dto.source_ip, ""); // default empty string
        assert_eq!(dto.source_port, 0); // default 0
        assert_eq!(dto.protocol, "unknown"); // default "unknown"
    }
}
