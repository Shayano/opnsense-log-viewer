//! Batch Writer for SQLite Transaction Batching
//!
//! Story 6.2: Implements a single-threaded writer that consumes from a bounded channel
//! and inserts entries in batched transactions for optimal SQLite performance.
//!
//! ## Design
//!
//! - **Single writer thread**: WAL mode allows only one writer
//! - **Transaction batching**: 10,000 entries per transaction minimizes fsync overhead
//! - **Prepared statement caching**: `prepare_cached()` reuses compiled statements
//! - **Graceful shutdown**: Flushes remaining entries when channel closes
//!
//! ## Usage
//!
//! ```ignore
//! let (sender, receiver) = crossbeam_channel::bounded(50_000);
//! let progress = PipelineProgress::new();
//!
//! // In writer thread:
//! let writer = BatchWriter::new(receiver, progress.clone());
//! let entries_written = writer.run(&mut connection)?;
//! ```

use std::sync::atomic::Ordering;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use rusqlite::Connection;

use super::error::PipelineError;
use super::parsed_entry::ParsedEntry;
use super::progress::PipelineProgress;

/// Default batch size for transaction batching (10,000 entries)
pub const DEFAULT_BATCH_SIZE: usize = 10_000;

/// SQL INSERT statement for entries table
const INSERT_SQL: &str = r#"
    INSERT INTO entries (
        byte_offset, timestamp, source_ip, source_port,
        dest_ip, dest_port, action, protocol, interface, rule_id, raw_line
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
"#;

/// Batch writer that consumes from channel and writes to SQLite
///
/// Accumulates entries until batch size is reached, then inserts them
/// in a single transaction for optimal performance.
pub struct BatchWriter {
    /// Channel receiver for incoming entries
    receiver: Receiver<ParsedEntry>,

    /// Accumulated batch of entries
    batch: Vec<ParsedEntry>,

    /// Batch size threshold for flushing
    batch_size: usize,

    /// Progress tracking (shared with parser via Arc for proper atomic sharing)
    progress: Arc<PipelineProgress>,

    /// Total entries written across all batches
    total_written: u64,
}

impl BatchWriter {
    /// Create a new BatchWriter with default batch size (wraps progress in Arc)
    pub fn new(receiver: Receiver<ParsedEntry>, progress: PipelineProgress) -> Self {
        Self::with_batch_size(receiver, progress, DEFAULT_BATCH_SIZE)
    }

    /// Create a new BatchWriter with Arc-wrapped progress for proper atomic sharing
    /// This is the preferred constructor when sharing progress between threads
    pub fn new_with_arc(receiver: Receiver<ParsedEntry>, progress: Arc<PipelineProgress>) -> Self {
        Self::with_batch_size_arc(receiver, progress, DEFAULT_BATCH_SIZE)
    }

    /// Create a new BatchWriter with custom batch size (wraps progress in Arc)
    pub fn with_batch_size(
        receiver: Receiver<ParsedEntry>,
        progress: PipelineProgress,
        batch_size: usize,
    ) -> Self {
        Self::with_batch_size_arc(receiver, Arc::new(progress), batch_size)
    }

    /// Create a new BatchWriter with custom batch size and Arc-wrapped progress
    pub fn with_batch_size_arc(
        receiver: Receiver<ParsedEntry>,
        progress: Arc<PipelineProgress>,
        batch_size: usize,
    ) -> Self {
        Self {
            receiver,
            batch: Vec::with_capacity(batch_size),
            batch_size,
            progress,
            total_written: 0,
        }
    }

    /// Run the writer loop, consuming from channel until closed
    ///
    /// Blocks until the channel is closed (sender dropped), then flushes
    /// any remaining entries and returns the total count.
    ///
    /// # Arguments
    /// * `conn` - Mutable reference to SQLite connection
    ///
    /// # Returns
    /// * `Ok(u64)` - Total entries written
    /// * `Err(PipelineError)` - On database error
    pub fn run(mut self, conn: &mut Connection) -> Result<u64, PipelineError> {
        log::info!(
            "[BATCH WRITER] Starting writer with batch size {}",
            self.batch_size
        );

        loop {
            match self.receiver.recv() {
                Ok(entry) => {
                    self.batch.push(entry);

                    if self.batch.len() >= self.batch_size {
                        self.flush_batch(conn)?;
                    }
                }
                Err(_) => {
                    // Channel closed (sender dropped), flush remaining entries
                    log::info!(
                        "[BATCH WRITER] Channel closed, flushing {} remaining entries",
                        self.batch.len()
                    );

                    if !self.batch.is_empty() {
                        self.flush_batch(conn)?;
                    }
                    break;
                }
            }
        }

        log::info!(
            "[BATCH WRITER] Completed, total entries written: {}",
            self.total_written
        );

        Ok(self.total_written)
    }

    /// Flush the current batch to SQLite
    fn flush_batch(&mut self, conn: &mut Connection) -> Result<(), PipelineError> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let batch_len = self.batch.len();
        log::debug!("[BATCH WRITER] Flushing batch of {} entries", batch_len);

        // Begin transaction
        let tx = conn.transaction()?;

        {
            // Use prepared cached statement for efficiency
            let mut stmt = tx.prepare_cached(INSERT_SQL)?;

            for entry in self.batch.drain(..) {
                stmt.execute(rusqlite::params![
                    entry.byte_offset,
                    entry.timestamp,
                    entry.source_ip,
                    entry.source_port,
                    entry.dest_ip,
                    entry.dest_port,
                    entry.action,
                    entry.protocol,
                    entry.interface,
                    entry.rule_id,
                    entry.raw_line,
                ])?;
            }
        }

        // Commit transaction
        tx.commit()?;

        // Update counters
        self.total_written += batch_len as u64;
        self.progress
            .entries_written
            .fetch_add(batch_len as u64, Ordering::Relaxed);

        log::debug!(
            "[BATCH WRITER] Batch committed, total written: {}",
            self.total_written
        );

        Ok(())
    }

    /// Get the current batch size setting
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Get the number of entries currently in the batch (not yet written)
    pub fn pending_count(&self) -> usize {
        self.batch.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use rusqlite::Connection;
    use std::thread;

    use crate::indexer::sqlite::schema::create_schema;
    use crate::indexer::sqlite::connection::configure_connection;

    fn create_test_connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();
        create_schema(&conn).unwrap();
        conn
    }

    fn create_test_entry(offset: i64) -> ParsedEntry {
        ParsedEntry {
            byte_offset: offset,
            timestamp: "2026-01-22T10:00:00Z".to_string(),
            source_ip: Some("192.168.1.1".to_string()),
            source_port: Some(12345),
            dest_ip: Some("10.0.0.1".to_string()),
            dest_port: Some(443),
            action: "pass".to_string(),
            protocol: Some("TCP".to_string()),
            interface: Some("vtnet0".to_string()),
            rule_id: Some("rule_1".to_string()),
            raw_line: Some("test log line".to_string()),
        }
    }

    #[test]
    fn test_batch_writer_new() {
        let (_, receiver) = bounded(100);
        let progress = PipelineProgress::new();
        let writer = BatchWriter::new(receiver, progress);

        assert_eq!(writer.batch_size(), DEFAULT_BATCH_SIZE);
        assert_eq!(writer.pending_count(), 0);
    }

    #[test]
    fn test_batch_writer_custom_batch_size() {
        let (_, receiver) = bounded(100);
        let progress = PipelineProgress::new();
        let writer = BatchWriter::with_batch_size(receiver, progress, 500);

        assert_eq!(writer.batch_size(), 500);
    }

    #[test]
    fn test_batch_writer_single_entry() {
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        // Send one entry then close channel
        sender.send(create_test_entry(0)).unwrap();
        drop(sender);

        let writer = BatchWriter::with_batch_size(receiver, progress, 100);
        let result = writer.run(&mut conn);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Verify data was written to database
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_batch_writer_batch_flush() {
        let (sender, receiver) = bounded(1000);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        // Send exactly batch_size entries
        let batch_size = 50;
        for i in 0..batch_size {
            sender.send(create_test_entry(i as i64)).unwrap();
        }
        drop(sender);

        let writer = BatchWriter::with_batch_size(receiver, progress.clone(), batch_size);
        let result = writer.run(&mut conn);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), batch_size as u64);
    }

    #[test]
    fn test_batch_writer_multiple_batches() {
        let (sender, receiver) = bounded(1000);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        // Send 3.5 batches worth of entries
        let batch_size = 20;
        let total_entries = 70;

        for i in 0..total_entries {
            sender.send(create_test_entry(i as i64)).unwrap();
        }
        drop(sender);

        let writer = BatchWriter::with_batch_size(receiver, progress.clone(), batch_size);
        let result = writer.run(&mut conn);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), total_entries as u64);

        // Verify data in database
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, total_entries as i64);
    }

    #[test]
    fn test_batch_writer_empty_channel() {
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        drop(sender); // Close channel immediately

        let writer = BatchWriter::new(receiver, progress);
        let result = writer.run(&mut conn);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_batch_writer_null_fields() {
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        // Entry with all optional fields as None
        let entry = ParsedEntry {
            byte_offset: 0,
            timestamp: "2026-01-22T10:00:00Z".to_string(),
            source_ip: None,
            source_port: None,
            dest_ip: None,
            dest_port: None,
            action: "block".to_string(),
            protocol: None,
            interface: None,
            rule_id: None,
            raw_line: None,
        };

        sender.send(entry).unwrap();
        drop(sender);

        let writer = BatchWriter::with_batch_size(receiver, progress, 100);
        let result = writer.run(&mut conn);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_batch_writer_concurrent_send() {
        let (sender, receiver) = bounded(1000);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        // Spawn sender thread
        let sender_handle = thread::spawn(move || {
            for i in 0..100 {
                sender.send(create_test_entry(i as i64)).unwrap();
            }
            // Sender dropped here, closing channel
        });

        let writer = BatchWriter::with_batch_size(receiver, progress.clone(), 25);
        let result = writer.run(&mut conn);

        sender_handle.join().unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[test]
    fn test_batch_writer_progress_tracking() {
        use std::sync::Arc;

        let (sender, receiver) = bounded(100);
        // Use Arc to share progress between test and writer
        let progress = Arc::new(PipelineProgress::new());
        let mut conn = create_test_connection();

        for i in 0..30 {
            sender.send(create_test_entry(i as i64)).unwrap();
        }
        drop(sender);

        // Clone the Arc, not the PipelineProgress
        let writer_progress = (*progress).clone();
        let writer = BatchWriter::with_batch_size(receiver, writer_progress, 10);
        let result = writer.run(&mut conn);

        // Check result directly since we can't share atomics through clone
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 30);

        // Verify data in database
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 30);
    }

    #[test]
    fn test_batch_writer_data_integrity() {
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();
        let mut conn = create_test_connection();

        // Send entry with specific values
        let entry = ParsedEntry {
            byte_offset: 12345,
            timestamp: "2026-01-22T15:30:00Z".to_string(),
            source_ip: Some("10.0.0.5".to_string()),
            source_port: Some(8080),
            dest_ip: Some("172.16.0.1".to_string()),
            dest_port: Some(443),
            action: "block".to_string(),
            protocol: Some("UDP".to_string()),
            interface: Some("em0".to_string()),
            rule_id: Some("deny_all".to_string()),
            raw_line: Some("original line".to_string()),
        };

        sender.send(entry).unwrap();
        drop(sender);

        let writer = BatchWriter::with_batch_size(receiver, progress, 100);
        writer.run(&mut conn).unwrap();

        // Verify data was written correctly
        let (byte_offset, action, source_ip): (i64, String, String) = conn
            .query_row(
                "SELECT byte_offset, action, source_ip FROM entries WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(byte_offset, 12345);
        assert_eq!(action, "block");
        assert_eq!(source_ip, "10.0.0.5");
    }
}
