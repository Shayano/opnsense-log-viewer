use crate::indexer::HybridIndex;
use crate::parser::parse_file_collect_matching;
use crate::query::executor::QueryExecutor;
use crate::query::types::{QueryError, QueryRequest, QueryResult};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use log::info;

/// DTO for frontend LogEntry (matches src/types/log-entry.ts)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryDto {
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

const MAX_ENTRIES_PER_FETCH: usize = 20_000;

lazy_static::lazy_static! {
    /// Global state for the hybrid index (shared with indexation commands)
    /// This is the shared index loaded during indexation and storage commands
    pub static ref HYBRID_INDEX: Arc<Mutex<Option<HybridIndex>>> = Arc::new(Mutex::new(None));
}

/// Execute a query against the loaded index
///
/// This command processes filter conditions with boolean logic (AND/OR/NOT)
/// and returns matching entry IDs.
///
/// # Arguments
/// * `request` - Query request containing filters and index hash
///
/// # Returns
/// * `Ok(QueryResult)` - Query results with entry IDs and execution time
/// * `Err(String)` - Error message if query execution fails
#[tauri::command]
pub async fn execute_query(request: QueryRequest) -> Result<QueryResult, String> {
    info!("Executing query with {} filters", request.filters.len());

    // Lock the hybrid index
    let index_guard = HYBRID_INDEX
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let hybrid_index = index_guard
        .as_ref()
        .ok_or_else(|| "Index not loaded. Please open a log file first.".to_string())?;

    // Verify index hash matches (ensures query runs against correct index)
    if !request.index_hash.is_empty() {
        if let Some(index_hash) = hybrid_index.source_file_hash() {
            if index_hash != request.index_hash {
                return Err(format!(
                    "Index hash mismatch. Expected '{}' but loaded index has '{}'. Please reload the log file.",
                    request.index_hash, index_hash
                ));
            }
        }
    }

    // Create query executor
    // NOTE: Cloning indexes is necessary because QueryExecutor takes ownership
    // Future optimization: Make QueryExecutor use references instead
    let mut executor = QueryExecutor::new(
        hybrid_index.inverted_index().clone(),
        hybrid_index.bitmap_index().clone(),
        hybrid_index.entry_count(),
    );

    // Execute query
    let result = executor.execute(request).map_err(|e| match e {
        QueryError::InvalidRegex(msg) => format!("Invalid regex pattern: {}", msg),
        QueryError::IndexNotFound => "Index not found or hash mismatch".to_string(),
        QueryError::NoFilters => "No filters provided".to_string(),
        QueryError::ExecutionError(msg) => format!("Query execution failed: {}", msg),
    })?;

    info!(
        "Query completed: {} matches ({}ms)",
        result.matched_count,
        result.execution_time_ms
    );
    log::info!(
        "[MEM] execute_query: entry_ids.len()={} matched_count={} total_count={} (entry_ids sent to frontend)",
        result.entry_ids.len(),
        result.matched_count,
        result.total_count
    );

    Ok(result)
}

/// Fetch full log entries by IDs for display in LogTable
///
/// Streams the source file line-by-line and collects only entries whose id is in
/// the requested set. Does not load the entire file into memory. Limited to
/// MAX_ENTRIES_PER_FETCH to avoid timeouts on large result sets.
#[tauri::command]
pub async fn get_entries_by_ids(entry_ids: Vec<u64>) -> Result<Vec<LogEntryDto>, String> {
    let requested = entry_ids.len();
    let id_set: HashSet<u64> = entry_ids
        .into_iter()
        .take(MAX_ENTRIES_PER_FETCH)
        .collect();

    log::info!(
        "[MEM] get_entries_by_ids: request_ids={} id_set={}",
        requested,
        id_set.len()
    );

    if id_set.is_empty() {
        return Ok(vec![]);
    }

    let (path, format) = {
        let index_guard = HYBRID_INDEX
            .lock()
            .map_err(|e| format!("Failed to lock index: {}", e))?;

        let hybrid = index_guard
            .as_ref()
            .ok_or_else(|| "Index not loaded. Please open a log file first.".to_string())?;

        let meta = hybrid
            .metadata()
            .ok_or_else(|| "Index has no metadata (source path unknown).".to_string())?;

        (meta.source_file_path.clone(), meta.format)
    };

    let dtos = tokio::task::spawn_blocking(move || {
        let entries = parse_file_collect_matching(&path, format, &id_set, MAX_ENTRIES_PER_FETCH)
            .map_err(|e| format!("Failed to parse log file: {}", e))?;

        let result: Vec<LogEntryDto> = entries
            .into_iter()
            .map(|entry| LogEntryDto {
                id: entry.id.to_string(),
                timestamp: entry.timestamp.to_rfc3339(),
                interface: entry.interface.unwrap_or_default(),
                source_ip: entry.source_ip.unwrap_or_default(),
                source_port: entry.source_port.unwrap_or(0),
                destination_ip: entry.dest_ip.unwrap_or_default(),
                destination_port: entry.dest_port.unwrap_or(0),
                protocol: entry
                    .protocol
                    .unwrap_or_else(|| "unknown".to_string())
                    .to_lowercase(),
                action: entry
                    .action
                    .unwrap_or_else(|| "pass".to_string())
                    .to_lowercase(),
                rule_label: entry.rule_label.unwrap_or_default(),
            })
            .collect();
        Ok::<_, String>(result)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    info!("Fetched {} entries for display", dtos.len());
    log::info!(
        "[MEM] get_entries_by_ids: done result_len={} (sent to frontend as Vec<LogEntryDto>)",
        dtos.len()
    );
    Ok(dtos)
}

/// Set the global hybrid index (called after indexation completes)
///
/// This function is called by the indexation commands to make the index
/// available for queries.
#[allow(dead_code)]
pub fn set_hybrid_index(index: HybridIndex) -> Result<(), String> {
    let mut index_guard = HYBRID_INDEX
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let mem = index.memory_usage();
    log::info!(
        "[MEM] set_hybrid_index: replacing HYBRID_INDEX, new index memory_usage≈{} bytes",
        mem
    );
    *index_guard = Some(index);
    Ok(())
}

/// Clear the global hybrid index
#[allow(dead_code)]
pub fn clear_hybrid_index() -> Result<(), String> {
    let mut index_guard = HYBRID_INDEX
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let prev_mem = index_guard.as_ref().map(|i| i.memory_usage()).unwrap_or(0);
    log::info!(
        "[MEM] clear_hybrid_index: dropping HYBRID_INDEX, was≈{} bytes",
        prev_mem
    );
    *index_guard = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::types::{FilterCondition, FilterField, FilterOperator, FilterValue};

    #[tokio::test]
    async fn test_execute_query_no_index() {
        let request = QueryRequest {
            filters: vec![FilterCondition {
                field: FilterField::Action,
                operator: FilterOperator::Equals,
                value: FilterValue::String("block".to_string()),
                logic: None,
            }],
            index_hash: "test".to_string(),
        };

        let result = execute_query(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Index not loaded"));
    }
}
