use crate::indexer::HybridIndex;
use crate::query::executor::QueryExecutor;
use crate::query::types::{QueryError, QueryRequest, QueryResult};
use std::sync::{Arc, Mutex};
use log::info;

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

    Ok(result)
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

    *index_guard = Some(index);
    Ok(())
}

/// Clear the global hybrid index
#[allow(dead_code)]
pub fn clear_hybrid_index() -> Result<(), String> {
    let mut index_guard = HYBRID_INDEX
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

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
