//! Pipeline Progress Tracking
//!
//! Story 6.2: Provides atomic counters for tracking parallel parsing pipeline progress.
//!
//! ## Usage
//!
//! ```ignore
//! let progress = PipelineProgress::new();
//!
//! // In parser threads:
//! progress.entries_parsed.fetch_add(1, Ordering::Relaxed);
//!
//! // In writer thread:
//! progress.entries_written.fetch_add(batch_size, Ordering::Relaxed);
//!
//! // Check progress:
//! let stats = progress.snapshot();
//! println!("{} entries/sec", stats.entries_per_second);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Pipeline progress tracking with atomic counters
///
/// All counters use `Relaxed` ordering for maximum performance.
/// Progress reads may be slightly stale but that's acceptable for progress display.
#[derive(Debug)]
pub struct PipelineProgress {
    /// Bytes read from source file
    pub bytes_read: AtomicU64,

    /// Entries successfully parsed by Rayon workers
    pub entries_parsed: AtomicU64,

    /// Entries written to SQLite by writer thread
    pub entries_written: AtomicU64,

    /// Parse errors encountered (non-fatal)
    pub errors: AtomicU64,

    /// Start time for elapsed calculation
    start_time: Instant,
}

impl PipelineProgress {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            bytes_read: AtomicU64::new(0),
            entries_parsed: AtomicU64::new(0),
            entries_written: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Get a snapshot of current progress
    pub fn snapshot(&self) -> ProgressSnapshot {
        let bytes_read = self.bytes_read.load(Ordering::Relaxed);
        let entries_parsed = self.entries_parsed.load(Ordering::Relaxed);
        let entries_written = self.entries_written.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;

        let entries_per_second = if elapsed_ms > 0 {
            (entries_written as f64 / (elapsed_ms as f64 / 1000.0)) as u64
        } else {
            0
        };

        let bytes_per_second = if elapsed_ms > 0 {
            (bytes_read as f64 / (elapsed_ms as f64 / 1000.0)) as u64
        } else {
            0
        };

        ProgressSnapshot {
            bytes_read,
            entries_parsed,
            entries_written,
            errors,
            elapsed_ms,
            entries_per_second,
            bytes_per_second,
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Calculate entries per second based on written count
    pub fn entries_per_second(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let written = self.entries_written.load(Ordering::Relaxed);

        if elapsed > 0.0 {
            (written as f64 / elapsed) as u64
        } else {
            0
        }
    }

    /// Reset all counters (for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.bytes_read.store(0, Ordering::Relaxed);
        self.entries_parsed.store(0, Ordering::Relaxed);
        self.entries_written.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
    }
}

impl Default for PipelineProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PipelineProgress {
    fn clone(&self) -> Self {
        // Create a new progress with copied values
        let new_progress = Self::new();
        new_progress.bytes_read.store(
            self.bytes_read.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        new_progress.entries_parsed.store(
            self.entries_parsed.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        new_progress.entries_written.store(
            self.entries_written.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        new_progress.errors.store(
            self.errors.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        new_progress
    }
}

/// Immutable snapshot of progress at a point in time
#[derive(Debug, Clone, Copy)]
pub struct ProgressSnapshot {
    /// Bytes read from source file
    pub bytes_read: u64,

    /// Entries successfully parsed
    pub entries_parsed: u64,

    /// Entries written to SQLite
    pub entries_written: u64,

    /// Parse errors encountered
    pub errors: u64,

    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,

    /// Calculated entries per second (based on written)
    pub entries_per_second: u64,

    /// Calculated bytes per second
    pub bytes_per_second: u64,
}

impl ProgressSnapshot {
    /// Calculate parse success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.entries_parsed + self.errors;
        if total > 0 {
            (self.entries_parsed as f64 / total as f64) * 100.0
        } else {
            100.0
        }
    }

    /// Check if parsing is complete (all parsed entries are written)
    pub fn is_complete(&self) -> bool {
        self.entries_written > 0 && self.entries_written >= self.entries_parsed
    }

    /// Calculate write queue depth (parsed but not yet written)
    pub fn queue_depth(&self) -> u64 {
        self.entries_parsed.saturating_sub(self.entries_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_progress_new() {
        let progress = PipelineProgress::new();

        assert_eq!(progress.bytes_read.load(Ordering::Relaxed), 0);
        assert_eq!(progress.entries_parsed.load(Ordering::Relaxed), 0);
        assert_eq!(progress.entries_written.load(Ordering::Relaxed), 0);
        assert_eq!(progress.errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_progress_increment() {
        let progress = PipelineProgress::new();

        progress.bytes_read.fetch_add(100, Ordering::Relaxed);
        progress.entries_parsed.fetch_add(10, Ordering::Relaxed);
        progress.entries_written.fetch_add(5, Ordering::Relaxed);
        progress.errors.fetch_add(1, Ordering::Relaxed);

        let snapshot = progress.snapshot();

        assert_eq!(snapshot.bytes_read, 100);
        assert_eq!(snapshot.entries_parsed, 10);
        assert_eq!(snapshot.entries_written, 5);
        assert_eq!(snapshot.errors, 1);
    }

    #[test]
    fn test_progress_snapshot() {
        let progress = PipelineProgress::new();
        progress.entries_parsed.store(1000, Ordering::Relaxed);
        progress.entries_written.store(1000, Ordering::Relaxed);
        progress.errors.store(5, Ordering::Relaxed);

        let snapshot = progress.snapshot();

        assert_eq!(snapshot.entries_parsed, 1000);
        assert_eq!(snapshot.entries_written, 1000);
        assert_eq!(snapshot.errors, 5);
        // Elapsed time should be present (any non-negative value is valid for u64)
        let _ = snapshot.elapsed_ms;
    }

    #[test]
    fn test_progress_entries_per_second() {
        let progress = PipelineProgress::new();
        progress.entries_written.store(1000, Ordering::Relaxed);

        // Wait a bit so we have non-zero elapsed time
        thread::sleep(Duration::from_millis(10));

        let eps = progress.entries_per_second();
        // Should be some non-zero value
        assert!(eps > 0 || progress.elapsed_ms() == 0);
    }

    #[test]
    fn test_snapshot_success_rate() {
        let progress = PipelineProgress::new();
        progress.entries_parsed.store(95, Ordering::Relaxed);
        progress.errors.store(5, Ordering::Relaxed);

        let snapshot = progress.snapshot();
        let rate = snapshot.success_rate();

        assert!((rate - 95.0).abs() < 0.1);
    }

    #[test]
    fn test_snapshot_success_rate_all_success() {
        let progress = PipelineProgress::new();
        progress.entries_parsed.store(100, Ordering::Relaxed);
        progress.errors.store(0, Ordering::Relaxed);

        let snapshot = progress.snapshot();

        assert!((snapshot.success_rate() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_snapshot_success_rate_empty() {
        let progress = PipelineProgress::new();
        let snapshot = progress.snapshot();

        // No entries = 100% success (nothing failed)
        assert!((snapshot.success_rate() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_snapshot_is_complete() {
        let progress = PipelineProgress::new();

        // Not complete initially
        let snapshot = progress.snapshot();
        assert!(!snapshot.is_complete());

        // Still not complete if nothing written
        progress.entries_parsed.store(100, Ordering::Relaxed);
        let snapshot = progress.snapshot();
        assert!(!snapshot.is_complete());

        // Complete when written >= parsed
        progress.entries_written.store(100, Ordering::Relaxed);
        let snapshot = progress.snapshot();
        assert!(snapshot.is_complete());
    }

    #[test]
    fn test_snapshot_queue_depth() {
        let progress = PipelineProgress::new();
        progress.entries_parsed.store(1000, Ordering::Relaxed);
        progress.entries_written.store(800, Ordering::Relaxed);

        let snapshot = progress.snapshot();

        assert_eq!(snapshot.queue_depth(), 200);
    }

    #[test]
    fn test_snapshot_queue_depth_no_underflow() {
        let progress = PipelineProgress::new();
        progress.entries_parsed.store(100, Ordering::Relaxed);
        progress.entries_written.store(200, Ordering::Relaxed); // More written than parsed (edge case)

        let snapshot = progress.snapshot();

        assert_eq!(snapshot.queue_depth(), 0); // Should saturate to 0
    }

    #[test]
    fn test_progress_clone() {
        let progress = PipelineProgress::new();
        progress.entries_parsed.store(500, Ordering::Relaxed);
        progress.entries_written.store(400, Ordering::Relaxed);

        let cloned = progress.clone();

        assert_eq!(
            cloned.entries_parsed.load(Ordering::Relaxed),
            500
        );
        assert_eq!(
            cloned.entries_written.load(Ordering::Relaxed),
            400
        );
    }

    #[test]
    fn test_progress_default() {
        let progress = PipelineProgress::default();

        assert_eq!(progress.bytes_read.load(Ordering::Relaxed), 0);
        assert_eq!(progress.entries_parsed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_progress_reset() {
        let progress = PipelineProgress::new();
        progress.bytes_read.store(1000, Ordering::Relaxed);
        progress.entries_parsed.store(500, Ordering::Relaxed);

        progress.reset();

        assert_eq!(progress.bytes_read.load(Ordering::Relaxed), 0);
        assert_eq!(progress.entries_parsed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_concurrent_updates() {
        use std::sync::Arc;

        let progress = Arc::new(PipelineProgress::new());
        let mut handles = vec![];

        // Spawn multiple threads updating concurrently
        for _ in 0..4 {
            let p = Arc::clone(&progress);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    p.entries_parsed.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            progress.entries_parsed.load(Ordering::Relaxed),
            4000
        );
    }
}
