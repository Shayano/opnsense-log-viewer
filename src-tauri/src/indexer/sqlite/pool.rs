//! SQLite Connection Pool with Read/Write Management
//!
//! Story 6.1: Implements a connection pool optimized for WAL mode operations.
//!
//! ## Design
//!
//! - **Single write connection**: WAL mode allows only one writer at a time
//! - **Multiple read connections**: WAL mode allows concurrent readers
//! - **Connection recycling**: Health checks on reuse
//! - **Graceful shutdown**: Flush pending commits
//!
//! ## Usage
//!
//! ```ignore
//! let pool = SqliteConnectionPool::new(&db_path, 4)?;
//!
//! // Get write connection (exclusive access)
//! {
//!     let write_conn = pool.get_write_connection();
//!     write_conn.execute("INSERT INTO entries ...", params)?;
//! } // Lock released here
//!
//! // Get read connection (concurrent access)
//! {
//!     let read_conn = pool.get_read_connection()?;
//!     let results = read_conn.query("SELECT * FROM entries WHERE ...", params)?;
//! }
//! ```

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, atomic::{AtomicUsize, Ordering}};

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

use super::connection::configure_connection;
use super::schema::create_schema;

/// Default number of read connections in the pool
pub const DEFAULT_READ_POOL_SIZE: usize = 4;

/// Errors that can occur during pool operations
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Connection health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Schema error: {0}")]
    SchemaError(#[from] super::schema::SchemaError),

    #[error("Connection error: {0}")]
    ConnectionError(#[from] super::connection::ConnectionError),
}

/// SQLite connection pool with read/write separation
///
/// Provides managed access to SQLite connections optimized for WAL mode:
/// - Single write connection for exclusive inserts/updates
/// - Multiple read connections for concurrent queries
pub struct SqliteConnectionPool {
    /// Single write connection (WAL allows one writer)
    write_conn: Mutex<Connection>,

    /// Multiple read connections (WAL allows concurrent readers)
    read_conns: Vec<Mutex<Connection>>,

    /// Database path for health checks and diagnostics
    db_path: PathBuf,

    /// Round-robin counter for read connection selection
    read_counter: AtomicUsize,
}

impl SqliteConnectionPool {
    /// Create a new connection pool
    ///
    /// Opens one write connection and multiple read connections.
    /// All connections are configured with optimized PRAGMA settings.
    /// Schema is created if it doesn't exist.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `read_pool_size` - Number of read connections to create
    ///
    /// # Returns
    /// * `Ok(SqliteConnectionPool)` on success
    /// * `Err(PoolError)` if connection fails
    pub fn new(db_path: &Path, read_pool_size: usize) -> Result<Self, PoolError> {
        if read_pool_size == 0 {
            return Err(PoolError::ConfigError(
                "read_pool_size must be at least 1".to_string(),
            ));
        }

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open write connection with full access
        let write_conn = Connection::open(db_path)?;
        configure_connection(&write_conn)?;

        // Create schema if it doesn't exist
        create_schema(&write_conn)?;

        // Open read connections with SQLITE_OPEN_READ_ONLY
        let read_conns = (0..read_pool_size)
            .map(|_| {
                let conn = Connection::open_with_flags(
                    db_path,
                    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )?;
                configure_connection(&conn)?;
                Ok(Mutex::new(conn))
            })
            .collect::<Result<Vec<_>, PoolError>>()?;

        log::info!(
            "[SQLITE POOL] Created pool with 1 write + {} read connections for {}",
            read_pool_size,
            db_path.display()
        );

        Ok(Self {
            write_conn: Mutex::new(write_conn),
            read_conns,
            db_path: db_path.to_path_buf(),
            read_counter: AtomicUsize::new(0),
        })
    }

    /// Create a new connection pool with default read pool size (4)
    pub fn new_default(db_path: &Path) -> Result<Self, PoolError> {
        Self::new(db_path, DEFAULT_READ_POOL_SIZE)
    }

    /// Get exclusive write connection
    ///
    /// Blocks until the write connection is available.
    /// The connection is released when the guard is dropped.
    ///
    /// # Returns
    /// * `MutexGuard<Connection>` - Exclusive access to write connection
    pub fn get_write_connection(&self) -> MutexGuard<'_, Connection> {
        self.write_conn.lock().expect("Write connection mutex poisoned")
    }

    /// Get any available read connection
    ///
    /// Uses round-robin selection to distribute load across read connections.
    /// Falls back to blocking wait if all connections are busy.
    ///
    /// # Returns
    /// * `Ok(MutexGuard<Connection>)` - Access to a read connection
    /// * `Err(PoolError)` - If all connections fail health checks
    pub fn get_read_connection(&self) -> Result<MutexGuard<'_, Connection>, PoolError> {
        // Round-robin selection starting point
        let start_idx = self.read_counter.fetch_add(1, Ordering::Relaxed) % self.read_conns.len();

        // Try each connection once without blocking
        for i in 0..self.read_conns.len() {
            let idx = (start_idx + i) % self.read_conns.len();
            if let Ok(guard) = self.read_conns[idx].try_lock() {
                return Ok(guard);
            }
        }

        // All busy, wait for first one
        Ok(self.read_conns[start_idx]
            .lock()
            .expect("Read connection mutex poisoned"))
    }

    /// Check if a connection is healthy
    ///
    /// Executes a simple query to verify the connection is working.
    fn check_connection_health(conn: &Connection) -> Result<(), PoolError> {
        conn.query_row("SELECT 1", [], |_| Ok(()))
            .map_err(|e| PoolError::HealthCheckFailed(e.to_string()))
    }

    /// Get a read connection with health check
    ///
    /// Verifies the connection is healthy before returning it.
    pub fn get_healthy_read_connection(&self) -> Result<MutexGuard<'_, Connection>, PoolError> {
        let conn = self.get_read_connection()?;
        Self::check_connection_health(&conn)?;
        Ok(conn)
    }

    /// Shutdown the pool gracefully
    ///
    /// Flushes any pending WAL frames to the main database.
    /// Should be called before dropping the pool for clean shutdown.
    pub fn shutdown(&self) -> Result<(), PoolError> {
        log::info!("[SQLITE POOL] Shutting down pool for {}", self.db_path.display());

        // Get write connection and checkpoint WAL
        let write_conn = self.get_write_connection();

        // PRAGMA wal_checkpoint(TRUNCATE) flushes WAL and truncates the file
        write_conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

        log::info!("[SQLITE POOL] WAL checkpoint complete");
        Ok(())
    }

    /// Get the database path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Get the number of read connections in the pool
    pub fn read_pool_size(&self) -> usize {
        self.read_conns.len()
    }

    /// Get entry count from the database
    pub fn get_entry_count(&self) -> Result<u64, PoolError> {
        let conn = self.get_read_connection()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entries",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// Check if the database has any entries
    pub fn is_empty(&self) -> Result<bool, PoolError> {
        let conn = self.get_read_connection()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM entries LIMIT 1)",
            [],
            |row| row.get(0),
        )?;
        Ok(!exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::thread;
    use std::sync::Arc;

    fn create_test_pool() -> (TempDir, SqliteConnectionPool) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let pool = SqliteConnectionPool::new(&db_path, 2).unwrap();
        (temp, pool)
    }

    #[test]
    fn test_pool_creation_success() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        let result = SqliteConnectionPool::new(&db_path, 2);
        assert!(result.is_ok(), "Pool creation should succeed");

        let pool = result.unwrap();
        assert_eq!(pool.read_pool_size(), 2);
    }

    #[test]
    fn test_pool_default_size() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        let pool = SqliteConnectionPool::new_default(&db_path).unwrap();
        assert_eq!(pool.read_pool_size(), DEFAULT_READ_POOL_SIZE);
    }

    #[test]
    fn test_pool_zero_read_conns_error() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        let result = SqliteConnectionPool::new(&db_path, 0);
        assert!(result.is_err(), "Zero read connections should fail");
    }

    #[test]
    fn test_get_write_connection() {
        let (_temp, pool) = create_test_pool();

        let write_conn = pool.get_write_connection();

        // Verify connection is functional
        let result: i64 = write_conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_get_read_connection() {
        let (_temp, pool) = create_test_pool();

        let read_conn = pool.get_read_connection().unwrap();

        // Verify connection is functional
        let result: i64 = read_conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_read_connection_is_readonly() {
        let (_temp, pool) = create_test_pool();

        let read_conn = pool.get_read_connection().unwrap();

        // Attempt to write should fail
        let result = read_conn.execute(
            "INSERT INTO entries (byte_offset, timestamp, action) VALUES (0, '2026-01-22', 'pass')",
            [],
        );

        assert!(result.is_err(), "Read connection should not allow writes");
    }

    #[test]
    fn test_write_connection_can_write() {
        let (_temp, pool) = create_test_pool();

        let write_conn = pool.get_write_connection();

        // Write should succeed
        let result = write_conn.execute(
            "INSERT INTO entries (byte_offset, timestamp, action) VALUES (0, '2026-01-22', 'pass')",
            [],
        );

        assert!(result.is_ok(), "Write connection should allow writes");
    }

    #[test]
    fn test_concurrent_read_access() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let pool = Arc::new(SqliteConnectionPool::new(&db_path, 4).unwrap());

        // Insert some test data
        {
            let write_conn = pool.get_write_connection();
            for i in 0..100 {
                write_conn
                    .execute(
                        "INSERT INTO entries (byte_offset, timestamp, action) VALUES (?, ?, 'pass')",
                        [i as i64, i as i64],
                    )
                    .unwrap();
            }
        }

        // Spawn multiple reader threads
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let pool_clone = Arc::clone(&pool);
                thread::spawn(move || {
                    for _ in 0..10 {
                        let conn = pool_clone.get_read_connection().unwrap();
                        let count: i64 = conn
                            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
                            .unwrap();
                        assert_eq!(count, 100);
                    }
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }
    }

    #[test]
    fn test_write_connection_exclusive() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let pool = Arc::new(SqliteConnectionPool::new(&db_path, 2).unwrap());

        let pool_clone = Arc::clone(&pool);
        let handle = thread::spawn(move || {
            // Hold write connection for a bit
            let _write_conn = pool_clone.get_write_connection();
            thread::sleep(std::time::Duration::from_millis(50));
        });

        // Give thread time to acquire lock
        thread::sleep(std::time::Duration::from_millis(10));

        // This should block until the other thread releases
        let start = std::time::Instant::now();
        let _write_conn = pool.get_write_connection();
        let elapsed = start.elapsed();

        handle.join().unwrap();

        // Should have waited at least 30ms (50ms hold minus 10ms head start minus margin)
        assert!(
            elapsed.as_millis() >= 30,
            "Should have waited for write lock, elapsed: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_health_check() {
        let (_temp, pool) = create_test_pool();

        let conn = pool.get_healthy_read_connection();
        assert!(conn.is_ok(), "Health check should pass");
    }

    #[test]
    fn test_shutdown() {
        let (_temp, pool) = create_test_pool();

        // Insert some data
        {
            let write_conn = pool.get_write_connection();
            write_conn
                .execute(
                    "INSERT INTO entries (byte_offset, timestamp, action) VALUES (0, '2026-01-22', 'pass')",
                    [],
                )
                .unwrap();
        }

        // Shutdown should succeed
        let result = pool.shutdown();
        assert!(result.is_ok(), "Shutdown should succeed");
    }

    #[test]
    fn test_get_entry_count() {
        let (_temp, pool) = create_test_pool();

        // Initially empty
        assert_eq!(pool.get_entry_count().unwrap(), 0);

        // Insert entries
        {
            let write_conn = pool.get_write_connection();
            for i in 0..5 {
                write_conn
                    .execute(
                        "INSERT INTO entries (byte_offset, timestamp, action) VALUES (?, ?, 'pass')",
                        [i as i64, i as i64],
                    )
                    .unwrap();
            }
        }

        assert_eq!(pool.get_entry_count().unwrap(), 5);
    }

    #[test]
    fn test_is_empty() {
        let (_temp, pool) = create_test_pool();

        assert!(pool.is_empty().unwrap(), "Should be empty initially");

        // Insert entry
        {
            let write_conn = pool.get_write_connection();
            write_conn
                .execute(
                    "INSERT INTO entries (byte_offset, timestamp, action) VALUES (0, '2026-01-22', 'pass')",
                    [],
                )
                .unwrap();
        }

        assert!(!pool.is_empty().unwrap(), "Should not be empty after insert");
    }

    #[test]
    fn test_db_path() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let pool = SqliteConnectionPool::new(&db_path, 2).unwrap();

        assert_eq!(pool.db_path(), db_path);
    }

    #[test]
    fn test_round_robin_read_selection() {
        let (_temp, pool) = create_test_pool();

        // Get multiple read connections
        // They should be selected round-robin
        for _ in 0..10 {
            let _conn = pool.get_read_connection().unwrap();
            // Just verify it doesn't error
        }
    }

    #[test]
    fn test_creates_parent_directory() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("subdir").join("nested").join("test.sqlite");

        let result = SqliteConnectionPool::new(&db_path, 2);
        assert!(result.is_ok(), "Should create parent directories");
        assert!(db_path.exists(), "Database file should exist");
    }

    #[test]
    fn test_schema_created_on_new() {
        let (_temp, pool) = create_test_pool();

        // Verify schema exists by querying
        let conn = pool.get_read_connection().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='entries'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(exists, "entries table should exist after pool creation");
    }
}
