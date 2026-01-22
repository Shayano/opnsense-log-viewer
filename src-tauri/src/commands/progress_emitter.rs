//! Progress Emitter for Real-Time UI Updates
//!
//! Story 6.3 AC7: Emits progress events to frontend at 1 Hz frequency.
//!
//! ## Design
//!
//! - Runs in dedicated `std::thread` (NOT Tokio) to avoid blocking async runtime
//! - Polls `PipelineProgress` atomics every 1 second
//! - Calculates rolling 5-second average for stable speed display
//! - Stops when pipeline completes (checks `is_complete()`)
//!
//! ## Usage
//!
//! ```ignore
//! let emitter = ProgressEmitter::new(app, progress, total_bytes);
//!
//! // Run in dedicated thread
//! std::thread::spawn(move || {
//!     emitter.run();
//! });
//! ```

use std::collections::VecDeque;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::indexer::progress::IndexProgress;
use crate::indexer::sqlite::PipelineProgress;

/// Progress event emission frequency (1 Hz = every 1 second)
const EMISSION_INTERVAL: Duration = Duration::from_secs(1);

/// Rolling window size for speed averaging (5 seconds)
const SPEED_WINDOW_SIZE: usize = 5;

/// Average line length estimate for entry count estimation (200 bytes for firewall logs)
const AVG_LINE_LENGTH: u64 = 200;

/// Emits progress events to frontend at 1 Hz
///
/// Story 6.3 AC7: Dedicated thread polls PipelineProgress atomics.
pub struct ProgressEmitter {
    /// Tauri app handle for event emission
    app: AppHandle,
    /// Shared progress tracker from pipeline
    progress: Arc<PipelineProgress>,
    /// Total file size in bytes
    total_bytes: u64,
    /// Estimated total entries based on file size
    total_entries_estimated: u64,
}

impl ProgressEmitter {
    /// Create a new progress emitter
    ///
    /// # Arguments
    /// * `app` - Tauri app handle for emitting events
    /// * `progress` - Shared progress tracker from pipeline
    /// * `total_bytes` - Total file size in bytes
    pub fn new(app: AppHandle, progress: Arc<PipelineProgress>, total_bytes: u64) -> Self {
        // Estimate entries based on average line length
        let total_entries_estimated = total_bytes / AVG_LINE_LENGTH;

        Self {
            app,
            progress,
            total_bytes,
            total_entries_estimated,
        }
    }

    /// Run the progress emitter (blocking)
    ///
    /// This method runs in a loop, emitting progress events every 1 second
    /// until the pipeline completes. Should be called from a dedicated thread.
    ///
    /// Story 6.3 AC7: 1 Hz polling with rolling average speed.
    pub fn run(self) {
        let start = Instant::now();
        let mut speed_history: VecDeque<u64> = VecDeque::with_capacity(SPEED_WINDOW_SIZE + 1);
        let mut last_entries_written = 0u64;
        let mut last_time = start;

        log::debug!(
            "[PROGRESS EMITTER] Starting 1 Hz emission for {} bytes ({} estimated entries)",
            self.total_bytes,
            self.total_entries_estimated
        );

        loop {
            // Sleep for 1 second (1 Hz frequency)
            thread::sleep(EMISSION_INTERVAL);

            // Get progress snapshot
            let snapshot = self.progress.snapshot();

            // Check if complete
            if snapshot.is_complete() {
                log::debug!(
                    "[PROGRESS EMITTER] Pipeline complete, emitting final progress and stopping"
                );
                self.emit_completion(snapshot, start.elapsed().as_secs_f64());
                break;
            }

            // Calculate instantaneous speed for this interval
            let now = Instant::now();
            let interval_secs = now.duration_since(last_time).as_secs_f64();
            let entries_delta = snapshot.entries_written.saturating_sub(last_entries_written);

            let instant_speed = if interval_secs > 0.0 {
                (entries_delta as f64 / interval_secs) as u64
            } else {
                0
            };

            // Update rolling speed average (5-second window)
            speed_history.push_back(instant_speed);
            if speed_history.len() > SPEED_WINDOW_SIZE {
                speed_history.pop_front();
            }

            let avg_speed: u64 = if !speed_history.is_empty() {
                speed_history.iter().sum::<u64>() / speed_history.len() as u64
            } else {
                0
            };

            // Calculate ETA based on rolling average
            let remaining_entries = self
                .total_entries_estimated
                .saturating_sub(snapshot.entries_written);
            let eta_seconds = if avg_speed > 0 {
                remaining_entries as f64 / avg_speed as f64
            } else {
                0.0
            };

            // Build progress event with all Story 6.3 fields
            let elapsed_secs = start.elapsed().as_secs_f64();
            let progress_event = IndexProgress::with_batch_info(
                snapshot.bytes_read,
                self.total_bytes,
                elapsed_secs,
                snapshot.entries_written,
                self.total_entries_estimated,
                snapshot.entries_written > 0, // partial_filter_available after first entries
                0, // batches_completed - N/A for streaming
                0, // total_batches - N/A for streaming
            );

            // Emit progress event
            let _ = self.app.emit("indexation-progress", &progress_event);

            log::trace!(
                "[PROGRESS EMITTER] Emitted progress: {}% ({}/{} entries, {} entries/sec avg, ETA: {:.1}s)",
                progress_event.percentage as u32,
                snapshot.entries_written,
                self.total_entries_estimated,
                avg_speed,
                eta_seconds
            );

            // Update tracking for next iteration
            last_entries_written = snapshot.entries_written;
            last_time = now;
        }
    }

    /// Emit final completion event with accurate statistics
    fn emit_completion(&self, snapshot: crate::indexer::sqlite::ProgressSnapshot, elapsed_secs: f64) {
        let final_progress = IndexProgress::with_batch_info(
            self.total_bytes,
            self.total_bytes,
            elapsed_secs,
            snapshot.entries_written,
            snapshot.entries_written, // Actual count now known (use written count)
            true,
            1,
            1,
        );

        // Emit final progress event
        let _ = self.app.emit("indexation-progress", &final_progress);

        // Also emit completion event
        let _ = self.app.emit("indexation-complete", &final_progress);

        log::debug!(
            "[PROGRESS EMITTER] Final emission: {} entries in {:.2}s ({} entries/sec)",
            snapshot.entries_written,
            elapsed_secs,
            snapshot.entries_per_second
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_interval() {
        assert_eq!(EMISSION_INTERVAL.as_secs(), 1);
    }

    #[test]
    fn test_speed_window_size() {
        assert_eq!(SPEED_WINDOW_SIZE, 5);
    }

    #[test]
    fn test_total_entries_estimation() {
        // 100 MB file / 200 bytes avg = 500,000 entries
        let estimated = 100_000_000u64 / AVG_LINE_LENGTH;
        assert_eq!(estimated, 500_000);
    }

    #[test]
    fn test_rolling_average_calculation() {
        let mut history: VecDeque<u64> = VecDeque::new();

        // Add 5 speed samples
        history.push_back(100);
        history.push_back(200);
        history.push_back(300);
        history.push_back(400);
        history.push_back(500);

        let avg: u64 = history.iter().sum::<u64>() / history.len() as u64;
        assert_eq!(avg, 300); // (100+200+300+400+500) / 5
    }

    #[test]
    fn test_rolling_window_maintains_size() {
        let mut history: VecDeque<u64> = VecDeque::with_capacity(SPEED_WINDOW_SIZE + 1);

        // Add more than window size
        for i in 0..10 {
            history.push_back(i as u64);
            if history.len() > SPEED_WINDOW_SIZE {
                history.pop_front();
            }
        }

        assert_eq!(history.len(), SPEED_WINDOW_SIZE);
        // Should contain last 5 values: 5, 6, 7, 8, 9
        assert_eq!(history.front(), Some(&5));
        assert_eq!(history.back(), Some(&9));
    }

    #[test]
    fn test_eta_calculation() {
        let remaining_entries: u64 = 1_000_000;
        let avg_speed: u64 = 100_000; // entries per second

        let eta_seconds = remaining_entries as f64 / avg_speed as f64;
        assert!((eta_seconds - 10.0).abs() < 0.001); // 10 seconds
    }

    #[test]
    fn test_eta_with_zero_speed() {
        let remaining_entries: u64 = 1_000_000;
        let avg_speed: u64 = 0;

        let eta_seconds = if avg_speed > 0 {
            remaining_entries as f64 / avg_speed as f64
        } else {
            0.0
        };

        assert_eq!(eta_seconds, 0.0);
    }
}
