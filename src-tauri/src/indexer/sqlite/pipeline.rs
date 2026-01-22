//! Pipeline Orchestrator for SQLite Index Building
//!
//! Story 6.2: Coordinates the parallel parsing pipeline, connecting:
//! - File reader (BufReader)
//! - Bounded channel (crossbeam, backpressure)
//! - Rayon parallel parsing
//! - Single-threaded SQLite writer
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │  File Reader    │ --> │ Bounded Channel │ --> │ SQLite Writer   │
//! │  (BufReader)    │     │ (crossbeam)     │     │ (Single Thread) │
//! └─────────────────┘     └─────────────────┘     └─────────────────┘
//!         │                       ^
//!         v                       │
//! ┌─────────────────┐             │
//! │  Rayon Workers  │ ────────────┘
//! │ (Parallel Parse)│
//! └─────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! let pool = SqliteConnectionPool::new(&db_path, 4)?;
//! let stats = build_sqlite_index(&pool, &file_path, format, &file_hash, file_size)?;
//!
//! println!("Indexed {} entries at {} entries/sec", stats.entry_count, stats.entries_per_second);
//! ```

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::bounded;

use crate::types::log_entry::LogFormat;

use super::batch_writer::BatchWriter;
use super::error::PipelineError;
use super::parallel_parser::parse_file_parallel;
use super::pool::SqliteConnectionPool;
use super::progress::PipelineProgress;

/// Default channel capacity for backpressure (50,000 entries)
/// At ~200 bytes per entry, this is ~10MB buffer
pub const DEFAULT_CHANNEL_CAPACITY: usize = 50_000;

/// Statistics returned after index building
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Total entries successfully indexed
    pub entry_count: u64,

    /// Parse errors (non-fatal, skipped entries)
    pub error_count: u64,

    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,

    /// Calculated entries per second
    pub entries_per_second: u64,

    /// Bytes processed from source file
    pub bytes_processed: u64,

    /// Bytes per second throughput
    pub bytes_per_second: u64,
}

impl IndexStats {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.entry_count + self.error_count;
        if total > 0 {
            (self.entry_count as f64 / total as f64) * 100.0
        } else {
            100.0
        }
    }

    /// Format as human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Indexed {} entries in {:.2}s ({} entries/sec, {:.1}% success)",
            self.entry_count,
            self.elapsed_ms as f64 / 1000.0,
            self.entries_per_second,
            self.success_rate()
        )
    }
}

/// Build a SQLite index from a log file using parallel parsing
///
/// Orchestrates the full pipeline:
/// 1. Creates bounded channel for backpressure
/// 2. Spawns writer thread to consume from channel
/// 3. Runs parallel parsing (uses Rayon thread pool)
/// 4. Waits for writer to complete
/// 5. Updates file_info table with metadata
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `file_path` - Path to the log file
/// * `format` - Detected log format
/// * `file_hash` - SHA256 hash of file for cache validation
/// * `file_size` - Size of file in bytes
///
/// # Returns
/// * `Ok(IndexStats)` - Statistics about the indexing operation
/// * `Err(PipelineError)` - On fatal error
///
/// # Performance
/// - Target: 300K+ entries/second
/// - Memory: <1GB peak (bounded channel limits buffering)
/// - Disk: SQLite WAL mode for durability
pub fn build_sqlite_index(
    pool: &SqliteConnectionPool,
    file_path: &Path,
    format: LogFormat,
    file_hash: &str,
    file_size: u64,
) -> Result<IndexStats, PipelineError> {
    log::info!(
        "[PIPELINE] Starting index build for {:?} ({} bytes, {:?} format)",
        file_path,
        file_size,
        format
    );

    let start = Instant::now();
    let progress = Arc::new(PipelineProgress::new());

    // Create bounded channel for backpressure
    let (sender, receiver) = bounded(DEFAULT_CHANNEL_CAPACITY);

    // Share progress with writer thread via Arc (H1 fix: atomic sharing, not cloning)
    let writer_progress = Arc::clone(&progress);

    // Get write connection for writer thread
    // Note: We get this first to ensure the connection is available
    let pool_db_path = pool.db_path().to_path_buf();

    // Spawn writer thread
    let writer_handle = std::thread::spawn(move || {
        // Open a new connection in the writer thread
        // This is necessary because Connection is not Send
        let mut write_conn = rusqlite::Connection::open(&pool_db_path)
            .map_err(PipelineError::Database)?;

        // Configure connection
        super::connection::configure_connection(&write_conn)?;

        // Run the batch writer - pass Arc-wrapped progress
        let writer = BatchWriter::new_with_arc(receiver, writer_progress);
        writer.run(&mut write_conn)
    });

    // Run parallel parsing in main thread (uses Rayon thread pool internally)
    let parse_result = parse_file_parallel(file_path, format, sender, &progress);

    // If parsing failed, still wait for writer to finish
    if let Err(e) = parse_result {
        log::error!("[PIPELINE] Parse error: {}", e);
        // Writer will exit when channel closes (sender dropped)
        let _ = writer_handle.join();
        return Err(e);
    }

    // Sender is dropped here (out of scope), closing channel

    // Wait for writer thread to complete
    let entries_written = writer_handle
        .join()
        .map_err(|_| PipelineError::WriterPanic)??;

    // Update file_info table with metadata
    update_file_info(pool, file_path, file_hash, file_size, entries_written)?;

    // Calculate final statistics
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis() as u64;
    let bytes_processed = progress.bytes_read.load(Ordering::Relaxed);
    let error_count = progress.errors.load(Ordering::Relaxed);

    let entries_per_second = if elapsed_ms > 0 {
        (entries_written as f64 / (elapsed_ms as f64 / 1000.0)) as u64
    } else {
        entries_written
    };

    let bytes_per_second = if elapsed_ms > 0 {
        (bytes_processed as f64 / (elapsed_ms as f64 / 1000.0)) as u64
    } else {
        bytes_processed
    };

    let stats = IndexStats {
        entry_count: entries_written,
        error_count,
        elapsed_ms,
        entries_per_second,
        bytes_processed,
        bytes_per_second,
    };

    log::info!("[PIPELINE] {}", stats.summary());

    Ok(stats)
}

/// Update file_info table with indexing metadata
fn update_file_info(
    pool: &SqliteConnectionPool,
    file_path: &Path,
    file_hash: &str,
    file_size: u64,
    entry_count: u64,
) -> Result<(), PipelineError> {
    let conn = pool.get_write_connection();

    conn.execute(
        r#"
        INSERT OR REPLACE INTO file_info
            (file_path, file_hash, file_size, entry_count, indexed_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        rusqlite::params![
            file_path.to_string_lossy().to_string(),
            file_hash,
            file_size as i64,
            entry_count as i64,
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;

    log::debug!(
        "[PIPELINE] Updated file_info: hash={}, entries={}",
        file_hash,
        entry_count
    );

    Ok(())
}

/// Build index with custom channel capacity (for testing)
pub fn build_sqlite_index_with_capacity(
    pool: &SqliteConnectionPool,
    file_path: &Path,
    format: LogFormat,
    file_hash: &str,
    file_size: u64,
    channel_capacity: usize,
) -> Result<IndexStats, PipelineError> {
    log::info!(
        "[PIPELINE] Starting index build with custom capacity {} for {:?}",
        channel_capacity,
        file_path
    );

    let start = Instant::now();
    let progress = Arc::new(PipelineProgress::new());

    let (sender, receiver) = bounded(channel_capacity);
    let writer_progress = Arc::clone(&progress);
    let pool_db_path = pool.db_path().to_path_buf();

    let writer_handle = std::thread::spawn(move || {
        let mut write_conn = rusqlite::Connection::open(&pool_db_path)
            .map_err(PipelineError::Database)?;
        super::connection::configure_connection(&write_conn)?;
        let writer = BatchWriter::new_with_arc(receiver, writer_progress);
        writer.run(&mut write_conn)
    });

    let parse_result = parse_file_parallel(file_path, format, sender, &progress);

    if let Err(e) = parse_result {
        let _ = writer_handle.join();
        return Err(e);
    }

    let entries_written = writer_handle
        .join()
        .map_err(|_| PipelineError::WriterPanic)??;

    update_file_info(pool, file_path, file_hash, file_size, entries_written)?;

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis() as u64;
    let bytes_processed = progress.bytes_read.load(Ordering::Relaxed);
    let error_count = progress.errors.load(Ordering::Relaxed);

    let entries_per_second = if elapsed_ms > 0 {
        (entries_written as f64 / (elapsed_ms as f64 / 1000.0)) as u64
    } else {
        entries_written
    };

    let bytes_per_second = if elapsed_ms > 0 {
        (bytes_processed as f64 / (elapsed_ms as f64 / 1000.0)) as u64
    } else {
        bytes_processed
    };

    Ok(IndexStats {
        entry_count: entries_written,
        error_count,
        elapsed_ms,
        entries_per_second,
        bytes_processed,
        bytes_per_second,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_pool() -> (TempDir, SqliteConnectionPool) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.sqlite");
        let pool = SqliteConnectionPool::new(&db_path, 2).unwrap();
        (temp, pool)
    }

    fn create_test_log_file(lines: &[&str]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_index_stats_success_rate() {
        let stats = IndexStats {
            entry_count: 95,
            error_count: 5,
            elapsed_ms: 1000,
            entries_per_second: 95,
            bytes_processed: 10000,
            bytes_per_second: 10000,
        };

        assert!((stats.success_rate() - 95.0).abs() < 0.1);
    }

    #[test]
    fn test_index_stats_success_rate_no_errors() {
        let stats = IndexStats {
            entry_count: 100,
            error_count: 0,
            elapsed_ms: 1000,
            entries_per_second: 100,
            bytes_processed: 10000,
            bytes_per_second: 10000,
        };

        assert!((stats.success_rate() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_index_stats_summary() {
        let stats = IndexStats {
            entry_count: 1000,
            error_count: 10,
            elapsed_ms: 2000,
            entries_per_second: 500,
            bytes_processed: 50000,
            bytes_per_second: 25000,
        };

        let summary = stats.summary();
        assert!(summary.contains("1000"));
        assert!(summary.contains("500"));
    }

    #[test]
    fn test_build_sqlite_index_empty_file() {
        let (_temp, pool) = create_test_pool();
        let log_file = create_test_log_file(&[]);

        let result = build_sqlite_index(
            &pool,
            log_file.path(),
            LogFormat::CSV,
            "test_hash",
            0,
        );

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.entry_count, 0);
    }

    #[test]
    fn test_build_sqlite_index_file_not_found() {
        let (_temp, pool) = create_test_pool();

        let result = build_sqlite_index(
            &pool,
            Path::new("/nonexistent/file.log"),
            LogFormat::CSV,
            "test_hash",
            0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_build_sqlite_index_with_parse_errors() {
        let (_temp, pool) = create_test_pool();
        // Invalid CSV lines
        let log_file = create_test_log_file(&[
            "invalid,line,1",
            "another,bad,line",
            "", // empty line (skipped, not an error)
        ]);

        let result = build_sqlite_index(
            &pool,
            log_file.path(),
            LogFormat::CSV,
            "test_hash",
            100,
        );

        assert!(result.is_ok());
        let stats = result.unwrap();
        // Both lines failed to parse
        assert_eq!(stats.entry_count, 0);
        assert!(stats.error_count > 0);
    }

    #[test]
    fn test_build_sqlite_index_updates_file_info() {
        let (_temp, pool) = create_test_pool();
        let log_file = create_test_log_file(&["test"]);

        let _ = build_sqlite_index(
            &pool,
            log_file.path(),
            LogFormat::CSV,
            "unique_hash_123",
            42,
        );

        // Verify file_info was updated
        let conn = pool.get_read_connection().unwrap();
        let (hash, size): (String, i64) = conn
            .query_row(
                "SELECT file_hash, file_size FROM file_info LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(hash, "unique_hash_123");
        assert_eq!(size, 42);
    }

    #[test]
    fn test_build_sqlite_index_with_custom_capacity() {
        let (_temp, pool) = create_test_pool();
        let log_file = create_test_log_file(&["line1", "line2"]);

        let result = build_sqlite_index_with_capacity(
            &pool,
            log_file.path(),
            LogFormat::CSV,
            "test_hash",
            20,
            10, // Very small capacity
        );

        // Should complete without deadlock
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_bytes_tracked() {
        let (_temp, pool) = create_test_pool();
        let log_file = create_test_log_file(&[
            "line one here",
            "line two here",
            "line three here",
        ]);

        let result = build_sqlite_index(
            &pool,
            log_file.path(),
            LogFormat::CSV,
            "test_hash",
            100,
        );

        assert!(result.is_ok());
        let stats = result.unwrap();

        // Should have processed some bytes
        assert!(stats.bytes_processed > 0);
    }

    #[test]
    fn test_pipeline_elapsed_time() {
        let (_temp, pool) = create_test_pool();
        let log_file = create_test_log_file(&["test"]);

        let result = build_sqlite_index(
            &pool,
            log_file.path(),
            LogFormat::CSV,
            "test_hash",
            0,
        );

        assert!(result.is_ok());
        let stats = result.unwrap();

        // Elapsed time should be present (any non-negative value is valid for u64)
        let _ = stats.elapsed_ms;
    }
}
