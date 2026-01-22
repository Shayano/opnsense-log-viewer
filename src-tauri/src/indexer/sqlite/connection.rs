//! SQLite Connection Configuration with PRAGMA Optimizations
//!
//! Story 6.1: Configures SQLite connections with performance-optimized PRAGMA settings.
//!
//! ## PRAGMA Settings Applied
//!
//! - `journal_mode = WAL`: Concurrent reads during write, crash recovery
//! - `synchronous = NORMAL`: Safe for WAL, avoids fsync per commit
//! - `cache_size = -64000`: 64MB cache (negative = KB)
//! - `mmap_size = 268435456`: 256MB memory-mapped I/O
//! - `temp_store = MEMORY`: Temp tables in memory
//! - `page_size = 4096`: Standard page size (must be set before any data operations)

use rusqlite::Connection;
use thiserror::Error;

/// Errors that can occur during connection configuration
#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("PRAGMA configuration failed: {0}")]
    PragmaFailed(String),
}

/// Configuration values for SQLite PRAGMA settings
pub struct PragmaConfig {
    /// Journal mode (default: WAL)
    pub journal_mode: &'static str,
    /// Synchronous mode (default: NORMAL)
    pub synchronous: &'static str,
    /// Cache size in KB (negative value, default: -64000 = 64MB)
    pub cache_size: i64,
    /// Memory-mapped I/O size in bytes (default: 256MB)
    pub mmap_size: i64,
    /// Temp store location (default: MEMORY)
    pub temp_store: &'static str,
    /// Page size in bytes (default: 4096)
    pub page_size: i64,
}

impl Default for PragmaConfig {
    fn default() -> Self {
        Self {
            journal_mode: "WAL",
            synchronous: "NORMAL",
            cache_size: -64000,        // 64MB (negative = KB)
            mmap_size: 268_435_456,    // 256MB
            temp_store: "MEMORY",
            page_size: 4096,
        }
    }
}

/// Configure a SQLite connection with performance-optimized PRAGMA settings
///
/// Applies the default PRAGMA configuration for optimal log querying performance.
///
/// # PRAGMA Settings
///
/// | Setting | Value | Purpose |
/// |---------|-------|---------|
/// | journal_mode | WAL | Concurrent reads during write |
/// | synchronous | NORMAL | Safe for WAL, no fsync per commit |
/// | cache_size | -64000 | 64MB cache |
/// | mmap_size | 268435456 | 256MB memory-mapped I/O |
/// | temp_store | MEMORY | Temp tables in memory |
/// | page_size | 4096 | Standard page size |
///
/// # Arguments
/// * `conn` - SQLite connection to configure
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ConnectionError)` if configuration fails
pub fn configure_connection(conn: &Connection) -> Result<(), ConnectionError> {
    configure_connection_with_config(conn, &PragmaConfig::default())
}

/// Configure a SQLite connection with custom PRAGMA settings
///
/// # Arguments
/// * `conn` - SQLite connection to configure
/// * `config` - Custom PRAGMA configuration
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ConnectionError)` if configuration fails
pub fn configure_connection_with_config(
    conn: &Connection,
    config: &PragmaConfig,
) -> Result<(), ConnectionError> {
    // Note: page_size should be set before any data operations
    // For existing databases, page_size change requires VACUUM
    conn.execute_batch(&format!("PRAGMA page_size = {};", config.page_size))?;

    // Set journal mode (WAL for concurrent reads)
    // Note: In-memory databases will report "memory" instead of "wal" - this is expected
    let journal_mode: String = conn.query_row(
        &format!("PRAGMA journal_mode = {};", config.journal_mode),
        [],
        |row| row.get(0),
    )?;

    // Only validate for file-based databases - in-memory always returns "memory"
    let is_in_memory = journal_mode.to_lowercase() == "memory";
    if !is_in_memory && journal_mode.to_uppercase() != config.journal_mode {
        return Err(ConnectionError::PragmaFailed(format!(
            "Failed to set journal_mode to {}, got {}",
            config.journal_mode, journal_mode
        )));
    }

    // Set synchronous mode
    conn.execute_batch(&format!("PRAGMA synchronous = {};", config.synchronous))?;

    // Set cache size (negative value = KB)
    conn.execute_batch(&format!("PRAGMA cache_size = {};", config.cache_size))?;

    // Set memory-mapped I/O size
    conn.execute_batch(&format!("PRAGMA mmap_size = {};", config.mmap_size))?;

    // Set temp store to memory
    conn.execute_batch(&format!("PRAGMA temp_store = {};", config.temp_store))?;

    Ok(())
}

/// Verify that PRAGMA settings are correctly applied
///
/// # Arguments
/// * `conn` - SQLite connection to verify
///
/// # Returns
/// * `Ok(PragmaValues)` with current PRAGMA values
/// * `Err(ConnectionError)` on database error
pub fn get_pragma_values(conn: &Connection) -> Result<PragmaValues, ConnectionError> {
    let journal_mode: String =
        conn.query_row("PRAGMA journal_mode;", [], |row| row.get(0))?;

    let synchronous: i64 =
        conn.query_row("PRAGMA synchronous;", [], |row| row.get(0))?;

    let cache_size: i64 =
        conn.query_row("PRAGMA cache_size;", [], |row| row.get(0))?;

    // mmap_size may return no rows for in-memory databases - default to 0
    let mmap_size: i64 = conn
        .query_row("PRAGMA mmap_size;", [], |row| row.get(0))
        .unwrap_or(0);

    let temp_store: i64 =
        conn.query_row("PRAGMA temp_store;", [], |row| row.get(0))?;

    let page_size: i64 =
        conn.query_row("PRAGMA page_size;", [], |row| row.get(0))?;

    Ok(PragmaValues {
        journal_mode,
        synchronous,
        cache_size,
        mmap_size,
        temp_store,
        page_size,
    })
}

/// Current PRAGMA values for a connection
#[derive(Debug, Clone)]
pub struct PragmaValues {
    /// Journal mode (e.g., "wal", "delete", "memory")
    pub journal_mode: String,
    /// Synchronous mode (0=OFF, 1=NORMAL, 2=FULL)
    pub synchronous: i64,
    /// Cache size (negative = KB, positive = pages)
    pub cache_size: i64,
    /// Memory-mapped I/O size in bytes
    pub mmap_size: i64,
    /// Temp store (0=DEFAULT, 1=FILE, 2=MEMORY)
    pub temp_store: i64,
    /// Page size in bytes
    pub page_size: i64,
}

impl PragmaValues {
    /// Check if the default configuration is applied
    pub fn matches_default(&self) -> bool {
        self.journal_mode.to_uppercase() == "WAL"
            && self.synchronous == 1  // NORMAL
            && self.cache_size == -64000
            && self.mmap_size == 268_435_456
            && self.temp_store == 2   // MEMORY
            && self.page_size == 4096
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configure_connection_success() {
        let conn = Connection::open_in_memory().unwrap();
        let result = configure_connection(&conn);
        assert!(result.is_ok(), "Connection configuration should succeed");
    }

    #[test]
    fn test_pragma_journal_mode_wal() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .unwrap();

        // In-memory databases may report "memory" instead of "wal"
        // This is expected behavior
        assert!(
            journal_mode == "wal" || journal_mode == "memory",
            "Journal mode should be wal or memory for in-memory db, got: {}",
            journal_mode
        );
    }

    #[test]
    fn test_pragma_synchronous_normal() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(synchronous, 1, "Synchronous should be NORMAL (1)");
    }

    #[test]
    fn test_pragma_cache_size() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();

        let cache_size: i64 = conn
            .query_row("PRAGMA cache_size;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(cache_size, -64000, "Cache size should be -64000 (64MB)");
    }

    #[test]
    fn test_pragma_mmap_size() {
        // mmap_size requires a file-based database to be properly testable
        // In-memory databases don't support mmap_size in the same way
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let conn = Connection::open(&db_path).unwrap();
        configure_connection(&conn).unwrap();

        let mmap_size: i64 = conn
            .query_row("PRAGMA mmap_size;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(mmap_size, 268_435_456, "mmap_size should be 256MB");
    }

    #[test]
    fn test_pragma_temp_store_memory() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();

        let temp_store: i64 = conn
            .query_row("PRAGMA temp_store;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(temp_store, 2, "temp_store should be MEMORY (2)");
    }

    #[test]
    fn test_pragma_page_size() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();

        let page_size: i64 = conn
            .query_row("PRAGMA page_size;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(page_size, 4096, "Page size should be 4096");
    }

    #[test]
    fn test_get_pragma_values() {
        // Use file-based database to test all PRAGMA values including mmap_size
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let conn = Connection::open(&db_path).unwrap();
        configure_connection(&conn).unwrap();

        let values = get_pragma_values(&conn).unwrap();

        assert_eq!(values.synchronous, 1);
        assert_eq!(values.cache_size, -64000);
        assert_eq!(values.mmap_size, 268_435_456);
        assert_eq!(values.temp_store, 2);
        assert_eq!(values.page_size, 4096);
    }

    #[test]
    fn test_pragma_config_default() {
        let config = PragmaConfig::default();

        assert_eq!(config.journal_mode, "WAL");
        assert_eq!(config.synchronous, "NORMAL");
        assert_eq!(config.cache_size, -64000);
        assert_eq!(config.mmap_size, 268_435_456);
        assert_eq!(config.temp_store, "MEMORY");
        assert_eq!(config.page_size, 4096);
    }

    #[test]
    fn test_configure_connection_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Configure twice - should not error
        configure_connection(&conn).unwrap();
        let result = configure_connection(&conn);

        assert!(result.is_ok(), "Configuration should be idempotent");
    }

    #[test]
    fn test_custom_pragma_config() {
        // Use file-based database to test custom mmap_size
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let conn = Connection::open(&db_path).unwrap();

        let custom_config = PragmaConfig {
            journal_mode: "WAL",
            synchronous: "NORMAL",
            cache_size: -32000,  // 32MB instead of 64MB
            mmap_size: 134_217_728,  // 128MB instead of 256MB
            temp_store: "MEMORY",
            page_size: 4096,
        };

        configure_connection_with_config(&conn, &custom_config).unwrap();

        let values = get_pragma_values(&conn).unwrap();
        assert_eq!(values.cache_size, -32000);
        assert_eq!(values.mmap_size, 134_217_728);
    }

    #[test]
    fn test_pragma_values_matches_default() {
        let values = PragmaValues {
            journal_mode: "wal".to_string(),
            synchronous: 1,
            cache_size: -64000,
            mmap_size: 268_435_456,
            temp_store: 2,
            page_size: 4096,
        };

        assert!(values.matches_default(), "Values should match default");
    }

    #[test]
    fn test_pragma_values_not_matches_default() {
        let values = PragmaValues {
            journal_mode: "delete".to_string(),
            synchronous: 2,  // FULL instead of NORMAL
            cache_size: -2000,
            mmap_size: 0,
            temp_store: 1,
            page_size: 1024,
        };

        assert!(!values.matches_default(), "Values should not match default");
    }
}
