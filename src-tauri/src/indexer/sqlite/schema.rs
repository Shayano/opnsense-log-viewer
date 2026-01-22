//! SQLite Schema Definition for Log Storage
//!
//! Story 6.1: Defines the database schema optimized for log storage and querying.
//! The schema is denormalized for query performance, with indexes on all filterable fields.

use rusqlite::Connection;
use thiserror::Error;

/// Errors that can occur during schema operations
#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

/// Create the database schema with all tables and indexes
///
/// # Tables Created
/// - `entries`: Core log entries (denormalized for query performance)
/// - `file_info`: File metadata for cache validation
///
/// # Indexes Created
/// - `idx_source_ip`: Fast filtering by source IP
/// - `idx_dest_ip`: Fast filtering by destination IP
/// - `idx_action`: Fast filtering by action (pass/block)
/// - `idx_protocol`: Fast filtering by protocol (TCP/UDP/ICMP)
/// - `idx_interface`: Fast filtering by interface
/// - `idx_timestamp`: Fast filtering/sorting by timestamp
/// - `idx_ports`: Composite index for port filtering
///
/// # Arguments
/// * `conn` - SQLite connection to create schema in
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(SchemaError)` if schema creation fails
pub fn create_schema(conn: &Connection) -> Result<(), SchemaError> {
    // Create entries table - denormalized for query performance
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY,
            byte_offset INTEGER NOT NULL,
            timestamp TEXT NOT NULL,
            source_ip TEXT,
            source_port INTEGER,
            dest_ip TEXT,
            dest_port INTEGER,
            action TEXT NOT NULL,
            protocol TEXT,
            interface TEXT,
            rule_id TEXT,
            raw_line TEXT
        )
        "#,
        [],
    )?;

    // Create file_info table for cache validation
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS file_info (
            id INTEGER PRIMARY KEY,
            file_path TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            entry_count INTEGER DEFAULT 0,
            indexed_at TEXT NOT NULL,
            UNIQUE(file_hash)
        )
        "#,
        [],
    )?;

    // Create indexes for fast filtering
    create_indexes(conn)?;

    Ok(())
}

/// Create all indexes for the entries table
fn create_indexes(conn: &Connection) -> Result<(), SchemaError> {
    // Individual column indexes for common filters
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_source_ip ON entries(source_ip)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_dest_ip ON entries(dest_ip)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_action ON entries(action)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_protocol ON entries(protocol)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_interface ON entries(interface)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_timestamp ON entries(timestamp)",
        [],
    )?;

    // Composite index for port filtering (both source and dest together)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ports ON entries(source_port, dest_port)",
        [],
    )?;

    Ok(())
}

/// Verify that the schema exists and is valid
///
/// Checks for existence of required tables and indexes.
///
/// # Arguments
/// * `conn` - SQLite connection to verify
///
/// # Returns
/// * `Ok(true)` if schema is valid
/// * `Ok(false)` if schema is missing or invalid
/// * `Err(SchemaError)` on database error
pub fn verify_schema(conn: &Connection) -> Result<bool, SchemaError> {
    // Check entries table exists
    let entries_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='entries'",
        [],
        |row| row.get(0),
    )?;

    if !entries_exists {
        return Ok(false);
    }

    // Check file_info table exists
    let file_info_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='file_info'",
        [],
        |row| row.get(0),
    )?;

    if !file_info_exists {
        return Ok(false);
    }

    // Check all required indexes exist
    let required_indexes = [
        "idx_source_ip",
        "idx_dest_ip",
        "idx_action",
        "idx_protocol",
        "idx_interface",
        "idx_timestamp",
        "idx_ports",
    ];

    for index_name in required_indexes {
        let index_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name=?",
            [index_name],
            |row| row.get(0),
        )?;

        if !index_exists {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Get the list of columns in the entries table
///
/// Useful for debugging and verification.
pub fn get_entries_columns(conn: &Connection) -> Result<Vec<String>, SchemaError> {
    let mut stmt = conn.prepare("PRAGMA table_info(entries)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_schema_success() {
        let conn = Connection::open_in_memory().unwrap();
        let result = create_schema(&conn);
        assert!(result.is_ok(), "Schema creation should succeed");
    }

    #[test]
    fn test_create_schema_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Create schema twice - should not error
        create_schema(&conn).unwrap();
        let result = create_schema(&conn);
        assert!(result.is_ok(), "Schema creation should be idempotent");
    }

    #[test]
    fn test_entries_table_exists() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='entries'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(exists, "entries table should exist");
    }

    #[test]
    fn test_file_info_table_exists() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='file_info'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(exists, "file_info table should exist");
    }

    #[test]
    fn test_entries_table_columns() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let columns = get_entries_columns(&conn).unwrap();

        let expected_columns = [
            "id",
            "byte_offset",
            "timestamp",
            "source_ip",
            "source_port",
            "dest_ip",
            "dest_port",
            "action",
            "protocol",
            "interface",
            "rule_id",
            "raw_line",
        ];

        for expected in expected_columns {
            assert!(
                columns.contains(&expected.to_string()),
                "entries table should have column: {}",
                expected
            );
        }
    }

    #[test]
    fn test_all_indexes_created() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let required_indexes = [
            "idx_source_ip",
            "idx_dest_ip",
            "idx_action",
            "idx_protocol",
            "idx_interface",
            "idx_timestamp",
            "idx_ports",
        ];

        for index_name in required_indexes {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name=?",
                    [index_name],
                    |row| row.get(0),
                )
                .unwrap();

            assert!(exists, "Index {} should exist", index_name);
        }
    }

    #[test]
    fn test_verify_schema_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        let result = verify_schema(&conn).unwrap();
        assert!(!result, "Empty database should fail schema verification");
    }

    #[test]
    fn test_verify_schema_after_creation() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        let result = verify_schema(&conn).unwrap();
        assert!(result, "Schema should pass verification after creation");
    }

    #[test]
    fn test_file_info_unique_constraint() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        // Insert first file_info
        conn.execute(
            "INSERT INTO file_info (file_path, file_hash, file_size, indexed_at) VALUES (?, ?, ?, ?)",
            ["path1", "hash123", "1000", "2026-01-22"],
        )
        .unwrap();

        // Try to insert duplicate hash - should fail
        let result = conn.execute(
            "INSERT INTO file_info (file_path, file_hash, file_size, indexed_at) VALUES (?, ?, ?, ?)",
            ["path2", "hash123", "2000", "2026-01-22"],
        );

        assert!(result.is_err(), "Duplicate file_hash should fail");
    }

    #[test]
    fn test_entries_insert_and_query() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        // Insert test entry
        conn.execute(
            r#"INSERT INTO entries
               (byte_offset, timestamp, source_ip, source_port, dest_ip, dest_port, action, protocol, interface, rule_id, raw_line)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            rusqlite::params![
                0i64,
                "2026-01-22T10:00:00Z",
                "192.168.1.100",
                12345,
                "10.0.0.1",
                443,
                "pass",
                "TCP",
                "vtnet0",
                "rule_1",
                "raw log line"
            ],
        )
        .unwrap();

        // Query by source_ip (uses index)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entries WHERE source_ip = ?",
                ["192.168.1.100"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1, "Should find 1 entry with matching source_ip");
    }

    #[test]
    fn test_entries_null_handling() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        // Insert entry with NULL optional fields
        conn.execute(
            r#"INSERT INTO entries
               (byte_offset, timestamp, source_ip, source_port, dest_ip, dest_port, action, protocol, interface, rule_id, raw_line)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            rusqlite::params![
                0i64,
                "2026-01-22T10:00:00Z",
                rusqlite::types::Null,  // source_ip can be NULL
                rusqlite::types::Null,  // source_port can be NULL
                rusqlite::types::Null,  // dest_ip can be NULL
                rusqlite::types::Null,  // dest_port can be NULL
                "block",                // action is NOT NULL
                rusqlite::types::Null,  // protocol can be NULL
                rusqlite::types::Null,  // interface can be NULL
                rusqlite::types::Null,  // rule_id can be NULL
                rusqlite::types::Null   // raw_line can be NULL
            ],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1, "Should successfully insert entry with NULL fields");
    }
}
