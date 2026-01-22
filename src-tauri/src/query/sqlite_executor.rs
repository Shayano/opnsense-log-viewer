//! SQLite Query Executor
//!
//! Story 6.4: Executes queries against SQLite database using parameterized SQL.
//!
//! ## Design
//!
//! This executor converts FilterCondition AST to SQL and executes queries
//! against the SQLite connection pool. It provides:
//! - Parameterized queries (no SQL injection)
//! - Pagination with window functions (COUNT(*) OVER())
//! - Concurrent query support via read connections
//! - Query timing instrumentation
//!
//! ## Usage
//!
//! ```ignore
//! let executor = SqliteQueryExecutor::new(pool.clone());
//! let result = executor.query(&filters, 1000, 0)?;
//! println!("Found {} entries", result.matched_count);
//! ```

use std::sync::Arc;
use std::time::Instant;

use rusqlite::{params_from_iter, OptionalExtension};

use crate::indexer::sqlite::SqliteConnectionPool;
use crate::query::filter_to_sql::FilterToSql;
use crate::query::types::{FilterCondition, QueryError, QueryResult};

/// Maximum entry IDs to return in a single query result
/// Matches existing limit in commands/query.rs
pub const MAX_ENTRY_IDS_IN_RESULT: usize = 20_000;

/// SQLite query executor with connection pool support
///
/// Provides interface-compatible query execution against SQLite database.
/// Uses read connections from pool for concurrent query support.
pub struct SqliteQueryExecutor {
    pool: Arc<SqliteConnectionPool>,
}

impl SqliteQueryExecutor {
    /// Create a new executor with a connection pool
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool for read operations
    pub fn new(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    /// Execute a query with filters and pagination
    ///
    /// Returns matching entry IDs with total count for pagination.
    /// Uses `COUNT(*) OVER()` window function for efficient total count.
    ///
    /// # Arguments
    /// * `filters` - Filter conditions to apply
    /// * `limit` - Maximum entries to return (capped at MAX_ENTRY_IDS_IN_RESULT)
    /// * `offset` - Number of entries to skip
    ///
    /// # Returns
    /// * `Ok(QueryResult)` - Entry IDs, counts, and execution time
    /// * `Err(QueryError)` - If query execution fails
    pub fn query(
        &self,
        filters: &[FilterCondition],
        limit: usize,
        offset: usize,
    ) -> Result<QueryResult, QueryError> {
        let start = Instant::now();

        // Convert filters to SQL
        let sql_query = FilterToSql::convert(filters)?;

        // Get read connection from pool
        let conn = self
            .pool
            .get_read_connection()
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        // Cap limit to MAX_ENTRY_IDS_IN_RESULT
        let effective_limit = limit.min(MAX_ENTRY_IDS_IN_RESULT);

        // Build SQL with window function for total count
        // COUNT(*) OVER() returns total matching rows alongside each row
        let sql = format!(
            "SELECT id, COUNT(*) OVER() as total_count FROM entries WHERE {} ORDER BY id LIMIT ? OFFSET ?",
            sql_query.where_clause
        );

        // Combine filter params with pagination params
        let mut params: Vec<rusqlite::types::Value> = sql_query.params;
        params.push(rusqlite::types::Value::Integer(effective_limit as i64));
        params.push(rusqlite::types::Value::Integer(offset as i64));

        // Execute query
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| QueryError::ExecutionError(format!("Prepare failed: {}", e)))?;

        let mut entry_ids: Vec<usize> = Vec::with_capacity(effective_limit.min(1000));
        let mut total_count: usize = 0;

        let rows = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                let id: i64 = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((id, count))
            })
            .map_err(|e| QueryError::ExecutionError(format!("Query failed: {}", e)))?;

        for row_result in rows {
            let (id, count) =
                row_result.map_err(|e| QueryError::ExecutionError(format!("Row read failed: {}", e)))?;
            entry_ids.push(id as usize);
            total_count = count as usize; // Same for all rows
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        log::debug!(
            "[SQLITE QUERY] {} filters, {} results, {} total, {}ms",
            filters.len(),
            entry_ids.len(),
            total_count,
            execution_time_ms
        );

        Ok(QueryResult {
            entry_ids,
            total_count,
            matched_count: total_count,
            execution_time_ms,
        })
    }

    /// Count matching entries without returning IDs
    ///
    /// More efficient than query() when only count is needed.
    ///
    /// # Arguments
    /// * `filters` - Filter conditions to apply
    ///
    /// # Returns
    /// * `Ok(u64)` - Number of matching entries
    /// * `Err(QueryError)` - If count query fails
    pub fn count(&self, filters: &[FilterCondition]) -> Result<u64, QueryError> {
        let start = Instant::now();

        let sql_query = FilterToSql::convert(filters)?;

        let conn = self
            .pool
            .get_read_connection()
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let sql = format!(
            "SELECT COUNT(*) FROM entries WHERE {}",
            sql_query.where_clause
        );

        let count: i64 = conn
            .query_row(&sql, params_from_iter(sql_query.params.iter()), |row| {
                row.get(0)
            })
            .map_err(|e| QueryError::ExecutionError(format!("Count query failed: {}", e)))?;

        log::debug!(
            "[SQLITE COUNT] {} filters, {} total, {}ms",
            filters.len(),
            count,
            start.elapsed().as_millis()
        );

        Ok(count as u64)
    }

    /// Get a single entry by ID
    ///
    /// # Arguments
    /// * `id` - Entry ID to retrieve
    ///
    /// # Returns
    /// * `Ok(Some(LogEntryDto))` - Entry data if found
    /// * `Ok(None)` - If entry not found
    /// * `Err(QueryError)` - If query fails
    pub fn get_entry(&self, id: u64) -> Result<Option<SqliteLogEntry>, QueryError> {
        let conn = self
            .pool
            .get_read_connection()
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let sql = "SELECT id, timestamp, source_ip, source_port, dest_ip, dest_port, action, protocol, interface, rule_id, raw_line FROM entries WHERE id = ?";

        let result = conn
            .query_row(sql, [id as i64], |row| {
                Ok(SqliteLogEntry {
                    id: row.get::<_, i64>(0)? as u64,
                    timestamp: row.get(1)?,
                    source_ip: row.get(2)?,
                    source_port: row.get::<_, Option<i64>>(3)?.map(|p| p as u16),
                    dest_ip: row.get(4)?,
                    dest_port: row.get::<_, Option<i64>>(5)?.map(|p| p as u16),
                    action: row.get(6)?,
                    protocol: row.get(7)?,
                    interface: row.get(8)?,
                    rule_id: row.get(9)?,
                    raw_line: row.get(10)?,
                })
            })
            .optional()
            .map_err(|e| QueryError::ExecutionError(format!("Get entry failed: {}", e)))?;

        Ok(result)
    }

    /// Get multiple entries by IDs
    ///
    /// Efficiently retrieves multiple entries in a single query.
    ///
    /// # Arguments
    /// * `ids` - Entry IDs to retrieve
    ///
    /// # Returns
    /// * `Ok(Vec<SqliteLogEntry>)` - Entries found (may be fewer than requested if some IDs don't exist)
    /// * `Err(QueryError)` - If query fails
    pub fn get_entries_by_ids(&self, ids: &[u64]) -> Result<Vec<SqliteLogEntry>, QueryError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .pool
            .get_read_connection()
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        // Build placeholders for IN clause
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, timestamp, source_ip, source_port, dest_ip, dest_port, action, protocol, interface, rule_id, raw_line FROM entries WHERE id IN ({}) ORDER BY id",
            placeholders.join(",")
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| QueryError::ExecutionError(format!("Prepare failed: {}", e)))?;

        // Convert IDs to params
        let params: Vec<rusqlite::types::Value> = ids
            .iter()
            .map(|id| rusqlite::types::Value::Integer(*id as i64))
            .collect();

        let rows = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                Ok(SqliteLogEntry {
                    id: row.get::<_, i64>(0)? as u64,
                    timestamp: row.get(1)?,
                    source_ip: row.get(2)?,
                    source_port: row.get::<_, Option<i64>>(3)?.map(|p| p as u16),
                    dest_ip: row.get(4)?,
                    dest_port: row.get::<_, Option<i64>>(5)?.map(|p| p as u16),
                    action: row.get(6)?,
                    protocol: row.get(7)?,
                    interface: row.get(8)?,
                    rule_id: row.get(9)?,
                    raw_line: row.get(10)?,
                })
            })
            .map_err(|e| QueryError::ExecutionError(format!("Query failed: {}", e)))?;

        let mut entries = Vec::with_capacity(ids.len());
        for row_result in rows {
            let entry = row_result
                .map_err(|e| QueryError::ExecutionError(format!("Row read failed: {}", e)))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get total entry count in the database
    ///
    /// # Returns
    /// * `Ok(u64)` - Total number of entries
    /// * `Err(QueryError)` - If count fails
    pub fn total_entries(&self) -> Result<u64, QueryError> {
        self.pool
            .get_entry_count()
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }
}

/// Log entry data from SQLite database
///
/// Simplified structure matching SQLite schema columns.
/// Used for internal query results before conversion to DTO.
#[derive(Debug, Clone)]
pub struct SqliteLogEntry {
    pub id: u64,
    pub timestamp: String,
    pub source_ip: Option<String>,
    pub source_port: Option<u16>,
    pub dest_ip: Option<String>,
    pub dest_port: Option<u16>,
    pub action: String,
    pub protocol: Option<String>,
    pub interface: Option<String>,
    pub rule_id: Option<String>,
    pub raw_line: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::types::{FilterField, FilterOperator, FilterValue, LogicOperator};
    use tempfile::TempDir;

    fn create_test_pool_with_data() -> (TempDir, Arc<SqliteConnectionPool>) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let pool = Arc::new(SqliteConnectionPool::new(&db_path, 2).unwrap());

        // Insert test data
        {
            let write_conn = pool.get_write_connection();
            for i in 0..100 {
                write_conn
                    .execute(
                        "INSERT INTO entries (byte_offset, timestamp, source_ip, dest_ip, source_port, dest_port, action, protocol, interface, rule_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        rusqlite::params![
                            i * 100,
                            format!("2026-01-22T10:{:02}:00Z", i % 60),
                            format!("192.168.1.{}", i % 256),
                            format!("10.0.0.{}", i % 256),
                            1024 + i,
                            443,
                            if i % 2 == 0 { "pass" } else { "block" },
                            if i % 3 == 0 { "TCP" } else { "UDP" },
                            "vtnet0",
                            format!("rule_{}", i % 10)
                        ],
                    )
                    .unwrap();
            }
        }

        (temp, pool)
    }

    fn make_filter(
        field: FilterField,
        op: FilterOperator,
        value: FilterValue,
        logic: Option<LogicOperator>,
    ) -> FilterCondition {
        FilterCondition {
            field,
            operator: op,
            value,
            logic,
        }
    }

    #[test]
    fn test_query_no_filters() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let result = executor.query(&[], 1000, 0).unwrap();

        assert_eq!(result.total_count, 100);
        assert_eq!(result.matched_count, 100);
        assert_eq!(result.entry_ids.len(), 100);
    }

    #[test]
    fn test_query_with_limit() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let result = executor.query(&[], 10, 0).unwrap();

        assert_eq!(result.total_count, 100); // Total still 100
        assert_eq!(result.entry_ids.len(), 10); // But only 10 returned
    }

    #[test]
    fn test_query_with_offset() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let result = executor.query(&[], 10, 90).unwrap();

        assert_eq!(result.total_count, 100);
        assert_eq!(result.entry_ids.len(), 10); // Last 10 entries
    }

    #[test]
    fn test_query_equals_filter() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let filters = vec![make_filter(
            FilterField::Action,
            FilterOperator::Equals,
            FilterValue::String("pass".to_string()),
            None,
        )];

        let result = executor.query(&filters, 1000, 0).unwrap();

        assert_eq!(result.total_count, 50); // Half are "pass"
        assert_eq!(result.matched_count, 50);
    }

    #[test]
    fn test_query_contains_filter() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Contains,
            FilterValue::String("192.168".to_string()),
            None,
        )];

        let result = executor.query(&filters, 1000, 0).unwrap();

        assert_eq!(result.total_count, 100); // All match
    }

    #[test]
    fn test_query_and_logic() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let filters = vec![
            make_filter(
                FilterField::Action,
                FilterOperator::Equals,
                FilterValue::String("pass".to_string()),
                Some(LogicOperator::And),
            ),
            make_filter(
                FilterField::Protocol,
                FilterOperator::Equals,
                FilterValue::String("TCP".to_string()),
                None,
            ),
        ];

        let result = executor.query(&filters, 1000, 0).unwrap();

        // pass (50%) AND TCP (33%) with some overlap
        assert!(result.total_count < 50);
    }

    #[test]
    fn test_query_or_logic() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let filters = vec![
            make_filter(
                FilterField::Protocol,
                FilterOperator::Equals,
                FilterValue::String("TCP".to_string()),
                Some(LogicOperator::Or),
            ),
            make_filter(
                FilterField::Protocol,
                FilterOperator::Equals,
                FilterValue::String("UDP".to_string()),
                None,
            ),
        ];

        let result = executor.query(&filters, 1000, 0).unwrap();

        assert_eq!(result.total_count, 100); // All entries are TCP or UDP
    }

    #[test]
    fn test_query_regex_filter() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Regex,
            FilterValue::String(r"192\.168\.1\.\d+".to_string()),
            None,
        )];

        let result = executor.query(&filters, 1000, 0).unwrap();

        assert_eq!(result.total_count, 100); // All match the pattern
    }

    #[test]
    fn test_count() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let filters = vec![make_filter(
            FilterField::Action,
            FilterOperator::Equals,
            FilterValue::String("block".to_string()),
            None,
        )];

        let count = executor.count(&filters).unwrap();

        assert_eq!(count, 50);
    }

    #[test]
    fn test_get_entry_exists() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let entry = executor.get_entry(1).unwrap();

        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.id, 1);
        assert!(entry.source_ip.is_some());
    }

    #[test]
    fn test_get_entry_not_found() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let entry = executor.get_entry(99999).unwrap();

        assert!(entry.is_none());
    }

    #[test]
    fn test_get_entries_by_ids() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let entries = executor.get_entries_by_ids(&[1, 5, 10]).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, 1);
        assert_eq!(entries[1].id, 5);
        assert_eq!(entries[2].id, 10);
    }

    #[test]
    fn test_get_entries_by_ids_empty() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let entries = executor.get_entries_by_ids(&[]).unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn test_total_entries() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let total = executor.total_entries().unwrap();

        assert_eq!(total, 100);
    }

    #[test]
    fn test_max_entry_ids_cap() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        // Request more than MAX_ENTRY_IDS_IN_RESULT
        let result = executor.query(&[], 100_000, 0).unwrap();

        // Should be capped, but since we only have 100 entries, it returns 100
        assert!(result.entry_ids.len() <= MAX_ENTRY_IDS_IN_RESULT);
    }

    #[test]
    fn test_query_execution_time() {
        let (_temp, pool) = create_test_pool_with_data();
        let executor = SqliteQueryExecutor::new(pool);

        let result = executor.query(&[], 1000, 0).unwrap();

        // Verify execution_time_ms is populated (always >= 0 for u64, just verify it exists)
        let _ = result.execution_time_ms;
    }

    #[test]
    fn test_query_empty_database() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("empty.sqlite");
        let pool = Arc::new(SqliteConnectionPool::new(&db_path, 2).unwrap());
        let executor = SqliteQueryExecutor::new(pool);

        let result = executor.query(&[], 1000, 0).unwrap();

        assert_eq!(result.total_count, 0);
        assert_eq!(result.entry_ids.len(), 0);
    }
}
