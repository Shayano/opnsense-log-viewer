//! Hash-Based SQLite Database Lookup and Management
//!
//! Story 6.1: Implements hash-based database caching for efficient log file indexation.
//!
//! ## Cache Strategy
//!
//! Each log file gets its own SQLite database stored at:
//! `{app_data}/log_indexes/{file_hash}.sqlite`
//!
//! - **file_hash**: SHA256(first 1MB + last 1MB + file size) - fast, unique identification
//! - **Hash match**: Return existing database (instant reload)
//! - **Hash mismatch**: Delete old database, create new
//!
//! ## Usage
//!
//! ```ignore
//! let pool = get_or_create_database(&app_handle, Path::new("/path/to/logfile.log"))?;
//!
//! // Pool is ready to use with schema created
//! let read_conn = pool.get_read_connection()?;
//! ```

use std::path::{Path, PathBuf};

use tauri::AppHandle;
use thiserror::Error;

use crate::storage::paths::{get_indexes_dir, PathError};
use crate::storage::{calculate_file_hash_quick, IntegrityError};
use crate::indexer::hybrid::IndexError;
use super::pool::{SqliteConnectionPool, PoolError, DEFAULT_READ_POOL_SIZE};

/// Errors that can occur during SQLite cache operations
#[derive(Error, Debug)]
pub enum SqliteCacheError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Pool error: {0}")]
    PoolError(#[from] PoolError),

    #[error("Path error: {0}")]
    PathError(#[from] PathError),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Hash calculation failed: {0}")]
    HashError(String),

    #[error("Index error: {0}")]
    IndexError(#[from] IndexError),

    #[error("Integrity error: {0}")]
    IntegrityError(#[from] IntegrityError),
}

/// Get or create a SQLite database for a log file
///
/// Uses file hash for cache lookup:
/// - If database exists and hash matches: return existing connection pool
/// - If database doesn't exist: create new with schema
/// - If hash differs: delete old database and create new
///
/// # Arguments
/// * `app_handle` - Tauri app handle for path resolution
/// * `file_path` - Path to the log file
///
/// # Returns
/// * `Ok(SqliteConnectionPool)` - Ready-to-use connection pool
/// * `Err(SqliteCacheError)` - If operation fails
pub fn get_or_create_database(
    app_handle: &AppHandle,
    file_path: &Path,
) -> Result<SqliteConnectionPool, SqliteCacheError> {
    // Verify file exists
    if !file_path.exists() {
        return Err(SqliteCacheError::FileNotFound(
            file_path.display().to_string(),
        ));
    }

    // Calculate file hash using shared function from storage::integrity
    // Story 6.5: Migrated from indexer/cache.rs to storage/integrity.rs
    let file_hash = calculate_file_hash_quick(file_path)?;
    let file_hash_hex = hex::encode(&file_hash);

    // Get database path
    let db_path = get_sqlite_database_path(app_handle, &file_hash_hex)?;

    log::debug!(
        "[SQLITE CACHE] Looking for database: {} (hash: {})",
        db_path.display(),
        &file_hash_hex[..16]
    );

    // Check if database exists
    if db_path.exists() {
        // Validate hash matches by checking file_info table
        match validate_existing_database(&db_path, &file_hash_hex) {
            Ok(true) => {
                log::info!(
                    "[SQLITE CACHE] Cache HIT - reusing database for {}",
                    file_path.display()
                );
                return Ok(SqliteConnectionPool::new(&db_path, DEFAULT_READ_POOL_SIZE)?);
            }
            Ok(false) => {
                log::info!(
                    "[SQLITE CACHE] Cache STALE - hash mismatch, recreating for {}",
                    file_path.display()
                );
                // Delete old database
                std::fs::remove_file(&db_path)?;
                // Also remove WAL and SHM files if they exist
                let _ = std::fs::remove_file(db_path.with_extension("sqlite-wal"));
                let _ = std::fs::remove_file(db_path.with_extension("sqlite-shm"));
            }
            Err(e) => {
                log::warn!(
                    "[SQLITE CACHE] Cache CORRUPT - validation failed: {}, recreating",
                    e
                );
                // Delete corrupted database
                let _ = std::fs::remove_file(&db_path);
                let _ = std::fs::remove_file(db_path.with_extension("sqlite-wal"));
                let _ = std::fs::remove_file(db_path.with_extension("sqlite-shm"));
            }
        }
    } else {
        log::info!(
            "[SQLITE CACHE] Cache MISS - creating new database for {}",
            file_path.display()
        );
    }

    // Create new database with schema
    let pool = SqliteConnectionPool::new(&db_path, DEFAULT_READ_POOL_SIZE)?;

    // Store file metadata in file_info table
    let file_size = std::fs::metadata(file_path)?.len();
    let indexed_at = chrono::Utc::now().to_rfc3339();

    {
        let write_conn = pool.get_write_connection();
        write_conn.execute(
            "INSERT OR REPLACE INTO file_info (file_path, file_hash, file_size, indexed_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                file_path.to_string_lossy().to_string(),
                file_hash_hex,
                file_size as i64,
                indexed_at
            ],
        )?;
    }

    Ok(pool)
}

/// Get the SQLite database path for a given file hash
///
/// # Arguments
/// * `app_handle` - Tauri app handle for path resolution
/// * `file_hash_hex` - Hex-encoded file hash
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the database file
/// * `Err(SqliteCacheError)` - If path resolution fails
pub fn get_sqlite_database_path(
    app_handle: &AppHandle,
    file_hash_hex: &str,
) -> Result<PathBuf, SqliteCacheError> {
    let indexes_dir = get_indexes_dir(app_handle)?;
    Ok(indexes_dir.join(format!("{}.sqlite", file_hash_hex)))
}

/// Validate that an existing database matches the expected hash
///
/// # Arguments
/// * `db_path` - Path to the database file
/// * `expected_hash` - Expected file hash (hex string)
///
/// # Returns
/// * `Ok(true)` - Database is valid and hash matches
/// * `Ok(false)` - Database exists but hash doesn't match
/// * `Err(PoolError)` - Database is corrupted or unreadable
fn validate_existing_database(db_path: &Path, expected_hash: &str) -> Result<bool, PoolError> {
    use rusqlite::Connection;

    let conn = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    // Check if file_info table exists and get stored hash
    let result: Result<String, _> = conn.query_row(
        "SELECT file_hash FROM file_info LIMIT 1",
        [],
        |row| row.get(0),
    );

    match result {
        Ok(stored_hash) => Ok(stored_hash == expected_hash),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            // Table exists but no rows - needs reindexing
            Ok(false)
        }
        Err(_) => {
            // Table doesn't exist or other error - database is invalid
            Ok(false)
        }
    }
}

// Story 6.5: calculate_file_hash_quick is now in crate::storage::integrity

/// Clear all SQLite database caches
///
/// Deletes all .sqlite files from the indexes directory.
///
/// # Arguments
/// * `app_handle` - Tauri app handle for path resolution
///
/// # Returns
/// * `Ok(u64)` - Number of database files deleted
/// * `Err(SqliteCacheError)` - If operation fails
pub fn clear_all_databases(app_handle: &AppHandle) -> Result<u64, SqliteCacheError> {
    let indexes_dir = get_indexes_dir(app_handle)?;

    if !indexes_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in std::fs::read_dir(&indexes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "sqlite").unwrap_or(false) {
            std::fs::remove_file(&path)?;
            // Also remove WAL and SHM files
            let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
            let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
            count += 1;
        }
    }

    log::info!("[SQLITE CACHE] Cleared {} database files", count);
    Ok(count)
}

/// Delete database for a specific file
///
/// # Arguments
/// * `app_handle` - Tauri app handle for path resolution
/// * `file_path` - Path to the log file whose database should be deleted
///
/// # Returns
/// * `Ok(bool)` - True if database was deleted, false if it didn't exist
/// * `Err(SqliteCacheError)` - If operation fails
pub fn delete_database_for_file(
    app_handle: &AppHandle,
    file_path: &Path,
) -> Result<bool, SqliteCacheError> {
    if !file_path.exists() {
        return Err(SqliteCacheError::FileNotFound(
            file_path.display().to_string(),
        ));
    }

    let file_hash = calculate_file_hash_quick(file_path)?;
    let file_hash_hex = hex::encode(&file_hash);
    let db_path = get_sqlite_database_path(app_handle, &file_hash_hex)?;

    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
        // Also remove WAL and SHM files
        let _ = std::fs::remove_file(db_path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(db_path.with_extension("sqlite-shm"));
        log::info!(
            "[SQLITE CACHE] Deleted database for {}",
            file_path.display()
        );
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Story 6.5: Hash calculation tests are in storage::integrity::tests

    #[test]
    fn test_validate_existing_database_valid() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        // Create database with file_info
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE file_info (
                    file_hash TEXT NOT NULL
                );
                INSERT INTO file_info (file_hash) VALUES ('abc123');
                "#,
            )
            .unwrap();
        }

        let result = validate_existing_database(&db_path, "abc123").unwrap();
        assert!(result, "Should validate when hash matches");
    }

    #[test]
    fn test_validate_existing_database_hash_mismatch() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        // Create database with different hash
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE file_info (
                    file_hash TEXT NOT NULL
                );
                INSERT INTO file_info (file_hash) VALUES ('abc123');
                "#,
            )
            .unwrap();
        }

        let result = validate_existing_database(&db_path, "different_hash").unwrap();
        assert!(!result, "Should fail when hash doesn't match");
    }

    #[test]
    fn test_validate_existing_database_empty_table() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        // Create database with empty file_info
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE file_info (
                    file_hash TEXT NOT NULL
                );
                "#,
            )
            .unwrap();
        }

        let result = validate_existing_database(&db_path, "any_hash").unwrap();
        assert!(!result, "Should fail when table is empty");
    }

    #[test]
    fn test_validate_existing_database_no_table() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");

        // Create empty database
        {
            let _conn = rusqlite::Connection::open(&db_path).unwrap();
        }

        let result = validate_existing_database(&db_path, "any_hash").unwrap();
        assert!(!result, "Should fail when table doesn't exist");
    }

    // Note: Integration tests with AppHandle require Tauri test context
    // These would be covered in integration tests
    //
    // The following functions can't be tested without AppHandle:
    // - get_or_create_database
    // - get_sqlite_database_path
    // - clear_all_databases
    // - delete_database_for_file
}
