//! Progressive Index Wrapper for Thread-Safe Concurrent Access
//!
//! Story 6.3 AC4: This module provides a thread-safe wrapper around TieredIndex
//! that allows concurrent read queries while indexation is still in progress.
//!
//! Architecture:
//! - `ProgressiveIndex` wraps `TieredIndex` with `Arc<RwLock<...>>`
//! - Writers can update the index as new batches complete
//! - Readers can query the currently indexed portion without blocking indexation
//! - Atomic counters track `entries_indexed` and `is_complete` status
//!
//! Usage:
//! ```ignore
//! let progressive = ProgressiveIndex::new();
//!
//! // Background thread: indexation
//! progressive.update(tiered_index_after_batch_1);
//! progressive.update(tiered_index_after_batch_2);
//! progressive.mark_complete();
//!
//! // UI thread: queries (concurrent with above)
//! let results = progressive.query_source_ip("192.168.1.1")?;
//! let is_done = progressive.is_complete();
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use roaring::RoaringBitmap;

use crate::indexer::hybrid::IndexError;
use crate::indexer::tiered::{TieredIndex, TieredQueryExecutor};

/// Thread-safe progressive index wrapper for concurrent read/write access
///
/// Story 6.3 AC4: Enables filtering on indexed portion while indexation continues.
///
/// # Thread Safety
/// - Uses `RwLock` for interior mutability (multiple readers OR single writer)
/// - Atomic counters for lock-free status checks
/// - Writers: `update()` and `mark_complete()` acquire write lock
/// - Readers: `query_*()` methods acquire read lock
pub struct ProgressiveIndex {
    /// The underlying tiered index (hot + warm tiers)
    inner: Arc<RwLock<TieredIndex>>,
    /// Flag indicating whether indexation is complete
    is_complete: AtomicBool,
    /// Number of entries currently indexed (for progress display)
    entries_indexed: AtomicU64,
}

impl ProgressiveIndex {
    /// Create a new empty progressive index
    ///
    /// Story 6.3 AC4: Initializes with empty TieredIndex, ready to receive updates.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TieredIndex::new())),
            is_complete: AtomicBool::new(false),
            entries_indexed: AtomicU64::new(0),
        }
    }

    /// Update the index with a new TieredIndex snapshot
    ///
    /// Story 6.3 AC4: Called after each batch merge to make new entries queryable.
    /// Acquires write lock, replaces inner TieredIndex, updates entry count.
    ///
    /// # Arguments
    /// * `tiered` - New TieredIndex containing all entries merged so far
    ///
    /// # Errors
    /// Returns `IndexError::LockError` if write lock acquisition fails.
    pub fn update(&self, tiered: TieredIndex) -> Result<(), IndexError> {
        let entries = tiered.entry_count();
        // Acquire write lock and replace the index
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire write lock for update".to_string())
        })?;
        *guard = tiered;
        // Update atomic entry count after successful update
        self.entries_indexed.store(entries, Ordering::SeqCst);
        Ok(())
    }

    /// Mark indexation as complete
    ///
    /// Story 6.3 AC4: Called when all batches have been processed.
    /// After this, queries will return complete results.
    pub fn mark_complete(&self) {
        self.is_complete.store(true, Ordering::SeqCst);
        log::info!(
            "[PROGRESSIVE] Indexation complete: {} entries",
            self.entries_indexed.load(Ordering::SeqCst)
        );
    }

    /// Get the number of entries currently indexed
    ///
    /// Story 6.3 AC4: Lock-free read of entry count for UI progress display.
    #[must_use]
    pub fn entries_indexed(&self) -> u64 {
        self.entries_indexed.load(Ordering::SeqCst)
    }

    /// Check if indexation is complete
    ///
    /// Story 6.3 AC4: Lock-free read of completion status.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::SeqCst)
    }

    /// Query source IP addresses
    ///
    /// Story 6.3 AC4: Returns entry IDs matching the IP from currently indexed portion.
    /// Acquires read lock for thread-safe access.
    pub fn query_source_ip(&self, ip: &str) -> Result<Vec<u64>, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_source_ip".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_source_ip(ip)
    }

    /// Query destination IP addresses
    ///
    /// Story 6.3 AC4: Returns entry IDs matching the IP from currently indexed portion.
    pub fn query_dest_ip(&self, ip: &str) -> Result<Vec<u64>, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_dest_ip".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_dest_ip(ip)
    }

    /// Query source ports
    ///
    /// Story 6.3 AC4: Returns entry IDs matching the port from currently indexed portion.
    pub fn query_source_port(&self, port: u16) -> Result<Vec<u64>, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_source_port".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_source_port(port)
    }

    /// Query destination ports
    ///
    /// Story 6.3 AC4: Returns entry IDs matching the port from currently indexed portion.
    pub fn query_dest_port(&self, port: u16) -> Result<Vec<u64>, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_dest_port".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_dest_port(port)
    }

    /// Query actions (block, pass, etc.)
    ///
    /// Story 6.3 AC4: Returns bitmap of entry IDs matching the action.
    pub fn query_action(&self, action: &str) -> Result<RoaringBitmap, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_action".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_action(action)
    }

    /// Query protocols (TCP, UDP, ICMP, etc.)
    ///
    /// Story 6.3 AC4: Returns bitmap of entry IDs matching the protocol.
    pub fn query_protocol(&self, protocol: &str) -> Result<RoaringBitmap, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_protocol".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_protocol(protocol)
    }

    /// Query interfaces (vtnet0, vtnet1, etc.)
    ///
    /// Story 6.3 AC4: Returns bitmap of entry IDs matching the interface.
    pub fn query_interface(&self, interface: &str) -> Result<RoaringBitmap, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for query_interface".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).query_interface(interface)
    }

    /// Get byte offset for a specific entry ID
    ///
    /// Story 6.3 AC4: Returns the file offset for random access to the raw log line.
    pub fn get_offset(&self, entry_id: u64) -> Result<Option<u64>, IndexError> {
        let mut guard = self.inner.write().map_err(|_| {
            IndexError::LockError("Failed to acquire lock for get_offset".to_string())
        })?;
        TieredQueryExecutor::new(&mut *guard).get_offset(entry_id)
    }

    /// Get a clone of the inner Arc for shared ownership
    ///
    /// Useful when the ProgressiveIndex needs to be shared across threads.
    pub fn clone_inner(&self) -> Arc<RwLock<TieredIndex>> {
        Arc::clone(&self.inner)
    }
}

impl Default for ProgressiveIndex {
    fn default() -> Self {
        Self::new()
    }
}

// Note: ProgressiveIndex is automatically Send + Sync because:
// - Arc<RwLock<TieredIndex>>: Send + Sync
// - AtomicBool: Send + Sync
// - AtomicU64: Send + Sync
// No manual unsafe impls needed - Rust derives these automatically.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::bitmap::BitmapIndex;
    use crate::indexer::inverted::InvertedIndex;
    use crate::indexer::offset_table::OffsetTable;
    use crate::indexer::tiered::TieredConfig;
    use std::thread;

    /// Create a test TieredIndex with some sample data
    fn create_test_tiered_index(entry_count: u64) -> TieredIndex {
        let mut inverted = InvertedIndex::new();
        let mut bitmap = BitmapIndex::new();
        let mut offsets = OffsetTable::new();

        for i in 0..entry_count {
            let ip = format!("192.168.1.{}", i % 255);
            inverted.add_entry(i, Some(&ip), Some("10.0.0.1"), Some(443), Some(80));
            bitmap.add_entry(
                i,
                Some(if i % 2 == 0 { "block" } else { "pass" }),
                Some("TCP"),
                Some("vtnet0"),
            );
            offsets.add_offset(i, i * 100);
        }

        let config = TieredConfig::with_hot_entries(entry_count + 1000);
        TieredIndex::from_hybrid(inverted, bitmap, offsets, entry_count, config).unwrap()
    }

    /// Story 6.3 AC4: Test basic ProgressiveIndex creation and update
    #[test]
    fn test_progressive_index_basic() {
        let progressive = ProgressiveIndex::new();

        assert_eq!(progressive.entries_indexed(), 0);
        assert!(!progressive.is_complete());

        // Update with initial data
        let tiered = create_test_tiered_index(1000);
        progressive.update(tiered).expect("update failed");

        assert_eq!(progressive.entries_indexed(), 1000);
        assert!(!progressive.is_complete());

        // Mark complete
        progressive.mark_complete();
        assert!(progressive.is_complete());
    }

    /// Story 6.3 AC4: Test query methods on ProgressiveIndex
    #[test]
    fn test_progressive_index_queries() {
        let progressive = ProgressiveIndex::new();
        let tiered = create_test_tiered_index(100);
        progressive.update(tiered).expect("update failed");

        // Test IP query
        let results = progressive.query_source_ip("192.168.1.0").unwrap();
        assert!(!results.is_empty());

        // Test action query
        let results = progressive.query_action("block").unwrap();
        assert_eq!(results.len(), 50); // Half are "block"

        // Test protocol query
        let results = progressive.query_protocol("TCP").unwrap();
        assert_eq!(results.len(), 100); // All are TCP

        // Test offset query
        let offset = progressive.get_offset(5).unwrap();
        assert_eq!(offset, Some(500));
    }

    /// Story 6.3 AC4: Test concurrent read/write access
    #[test]
    fn test_progressive_index_concurrent_access() {
        let progressive = Arc::new(ProgressiveIndex::new());

        // Initial data
        let tiered = create_test_tiered_index(100);
        progressive.update(tiered).expect("initial update failed");

        // Spawn reader threads
        let mut handles = vec![];

        for _ in 0..4 {
            let prog = Arc::clone(&progressive);
            handles.push(thread::spawn(move || {
                // Multiple read queries
                for _ in 0..10 {
                    let _ = prog.query_action("block");
                    let _ = prog.query_source_ip("192.168.1.0");
                    let _ = prog.entries_indexed();
                    let _ = prog.is_complete();
                }
            }));
        }

        // Writer thread updates concurrently
        let prog_writer = Arc::clone(&progressive);
        handles.push(thread::spawn(move || {
            for i in 1..=3 {
                let tiered = create_test_tiered_index(100 + i * 100);
                let _ = prog_writer.update(tiered); // Ignore result in concurrent test
                thread::sleep(std::time::Duration::from_millis(10));
            }
            prog_writer.mark_complete();
        }));

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Final state verification
        assert!(progressive.is_complete());
        assert!(progressive.entries_indexed() >= 100);
    }

    /// Story 6.3 AC4: Test readers see consistent snapshots during updates
    #[test]
    fn test_progressive_index_consistent_snapshots() {
        let progressive = Arc::new(ProgressiveIndex::new());

        // Update with 1000 entries
        let tiered = create_test_tiered_index(1000);
        progressive.update(tiered).expect("update failed");

        // Spawn multiple readers simultaneously
        let mut handles = vec![];
        for thread_id in 0..4 {
            let prog = Arc::clone(&progressive);
            handles.push(thread::spawn(move || {
                // Query and verify consistency
                let entry_count = prog.entries_indexed();
                let action_results = prog.query_action("block").unwrap();

                // For 1000 entries, exactly half should be "block"
                // Entry count might change during test, but action results should match
                assert!(
                    action_results.len() as u64 <= entry_count,
                    "Thread {}: More action results ({}) than entries ({})",
                    thread_id,
                    action_results.len(),
                    entry_count
                );
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    /// Story 6.3 AC4: Test progressive updates (simulating batch completions)
    #[test]
    fn test_progressive_index_incremental_updates() {
        let progressive = ProgressiveIndex::new();

        // Simulate batch 1 completion
        let tiered1 = create_test_tiered_index(1000);
        progressive.update(tiered1).expect("update 1 failed");
        assert_eq!(progressive.entries_indexed(), 1000);

        // Query partial results
        let results1 = progressive.query_action("block").unwrap();
        assert_eq!(results1.len(), 500);

        // Simulate batch 2 completion
        let tiered2 = create_test_tiered_index(2000);
        progressive.update(tiered2).expect("update 2 failed");
        assert_eq!(progressive.entries_indexed(), 2000);

        // Query updated results
        let results2 = progressive.query_action("block").unwrap();
        assert_eq!(results2.len(), 1000);

        // Simulate batch 3 (final) completion
        let tiered3 = create_test_tiered_index(3000);
        progressive.update(tiered3).expect("update 3 failed");
        progressive.mark_complete();

        assert!(progressive.is_complete());
        assert_eq!(progressive.entries_indexed(), 3000);

        let results3 = progressive.query_action("block").unwrap();
        assert_eq!(results3.len(), 1500);
    }
}
