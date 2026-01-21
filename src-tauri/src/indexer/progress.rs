use serde::{Deserialize, Serialize};

/// Progress information for indexation operations
///
/// Story 6.3: Extended for progressive indexation with early filtering support.
/// Tracks both byte-level progress and batch-level progress for UI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    /// Percentage of file processed (0-100)
    pub percentage: f64,
    /// Number of bytes processed so far
    pub bytes_processed: u64,
    /// Total file size in bytes
    pub total_bytes: u64,
    /// Processing speed in GB per minute
    pub speed_gbps: f64,
    /// Estimated time remaining in seconds
    pub eta_seconds: f64,
    /// Story 6.3 AC2: Number of log entries indexed so far
    pub entries_indexed: u64,
    /// Story 6.3 AC2: Estimated total entries (file_size / avg_line_length)
    pub total_entries_estimated: u64,
    /// Story 6.3 AC2: True when at least one batch is complete and filtering is available
    pub partial_filter_available: bool,
    /// Story 6.3 AC2: Number of batches that have completed processing
    pub batches_completed: u32,
    /// Story 6.3 AC2: Total number of batches to process
    pub total_batches: u32,
}

impl IndexProgress {
    /// Create a basic progress update (backwards compatible)
    ///
    /// Sets batch info fields to defaults (no partial filtering available).
    /// Use `with_batch_info()` for progressive indexation updates.
    pub fn new(bytes_processed: u64, total_bytes: u64, elapsed_seconds: f64) -> Self {
        Self::with_batch_info(
            bytes_processed,
            total_bytes,
            elapsed_seconds,
            0,     // entries_indexed
            0,     // total_entries_estimated
            false, // partial_filter_available
            0,     // batches_completed
            0,     // total_batches
        )
    }

    /// Create a progress update with batch information for progressive indexation
    ///
    /// Story 6.3 AC2: Full constructor with all progressive indexation fields.
    ///
    /// # Arguments
    /// * `bytes_processed` - Number of bytes processed so far
    /// * `total_bytes` - Total file size in bytes
    /// * `elapsed_seconds` - Time elapsed since start
    /// * `entries_indexed` - Number of log entries indexed
    /// * `total_entries_estimated` - Estimated total entries (file_size / avg_line_length)
    /// * `partial_filter_available` - True when filtering is available on indexed portion
    /// * `batches_completed` - Number of batches completed
    /// * `total_batches` - Total number of batches
    #[allow(clippy::too_many_arguments)]
    pub fn with_batch_info(
        bytes_processed: u64,
        total_bytes: u64,
        elapsed_seconds: f64,
        entries_indexed: u64,
        total_entries_estimated: u64,
        partial_filter_available: bool,
        batches_completed: u32,
        total_batches: u32,
    ) -> Self {
        let percentage = if total_bytes > 0 {
            (bytes_processed as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let speed_gbps = if elapsed_seconds > 0.0 {
            (bytes_processed as f64 / (1024.0 * 1024.0 * 1024.0)) / (elapsed_seconds / 60.0)
        } else {
            0.0
        };

        let bytes_remaining = total_bytes.saturating_sub(bytes_processed);
        let eta_seconds = if speed_gbps > 0.0 {
            (bytes_remaining as f64 / (1024.0 * 1024.0 * 1024.0)) / (speed_gbps / 60.0)
        } else {
            0.0
        };

        Self {
            percentage,
            bytes_processed,
            total_bytes,
            speed_gbps,
            eta_seconds,
            entries_indexed,
            total_entries_estimated,
            partial_filter_available,
            batches_completed,
            total_batches,
        }
    }

    /// Estimate total entries based on file size and average line length
    ///
    /// Story 6.3: Helper to calculate `total_entries_estimated` field.
    /// Uses 200 bytes as default average line length for firewall logs.
    pub fn estimate_entries(file_size: u64, avg_line_length: usize) -> u64 {
        let avg_len = if avg_line_length == 0 { 200 } else { avg_line_length };
        file_size / avg_len as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_calculation() {
        let progress = IndexProgress::new(500 * 1024 * 1024, 1024 * 1024 * 1024, 60.0);

        assert!((progress.percentage - 48.83).abs() < 0.1); // ~50%
        assert!(progress.speed_gbps > 0.0);
        assert!(progress.eta_seconds > 0.0);
        // Backwards compatible defaults
        assert_eq!(progress.entries_indexed, 0);
        assert_eq!(progress.total_entries_estimated, 0);
        assert!(!progress.partial_filter_available);
        assert_eq!(progress.batches_completed, 0);
        assert_eq!(progress.total_batches, 0);
    }

    #[test]
    fn test_progress_completion() {
        let progress = IndexProgress::new(1024 * 1024 * 1024, 1024 * 1024 * 1024, 120.0);

        assert!((progress.percentage - 100.0).abs() < 0.1);
        assert_eq!(progress.bytes_processed, progress.total_bytes);
    }

    /// Story 6.3 AC2: Test with_batch_info constructor
    #[test]
    fn test_progress_with_batch_info() {
        let progress = IndexProgress::with_batch_info(
            500_000_000,   // bytes_processed
            1_000_000_000, // total_bytes
            60.0,          // elapsed_seconds
            5_000_000,     // entries_indexed
            10_000_000,    // total_entries_estimated
            true,          // partial_filter_available
            1,             // batches_completed
            10,            // total_batches
        );

        // Verify base calculations still work
        assert!((progress.percentage - 50.0).abs() < 0.1);
        assert!(progress.speed_gbps > 0.0);
        assert!(progress.eta_seconds > 0.0);

        // Verify new fields
        assert_eq!(progress.entries_indexed, 5_000_000);
        assert_eq!(progress.total_entries_estimated, 10_000_000);
        assert!(progress.partial_filter_available);
        assert_eq!(progress.batches_completed, 1);
        assert_eq!(progress.total_batches, 10);
    }

    /// Story 6.3 AC2: Test partial_filter_available becomes true after first batch
    #[test]
    fn test_partial_filter_available_after_first_batch() {
        // Before first batch completes
        let progress_batch0 = IndexProgress::with_batch_info(
            100_000_000,
            1_000_000_000,
            10.0,
            0,
            10_000_000,
            false, // Not available yet
            0,
            10,
        );
        assert!(!progress_batch0.partial_filter_available);

        // After first batch completes
        let progress_batch1 = IndexProgress::with_batch_info(
            100_000_000,
            1_000_000_000,
            60.0,
            1_000_000,
            10_000_000,
            true, // Now available
            1,
            10,
        );
        assert!(progress_batch1.partial_filter_available);
        assert_eq!(progress_batch1.batches_completed, 1);
    }

    /// Story 6.3 AC2: Test batches_completed increments correctly
    #[test]
    fn test_batches_completed_increments() {
        for batch_num in 0..=5u32 {
            let progress = IndexProgress::with_batch_info(
                batch_num as u64 * 100_000_000,
                500_000_000,
                batch_num as f64 * 60.0,
                batch_num as u64 * 1_000_000,
                5_000_000,
                batch_num > 0,
                batch_num,
                5,
            );
            assert_eq!(progress.batches_completed, batch_num);
            assert_eq!(progress.total_batches, 5);
        }
    }

    /// Story 6.3 AC2: Test estimate_entries helper
    #[test]
    fn test_estimate_entries() {
        // 1GB file with 200 byte average lines
        let entries = IndexProgress::estimate_entries(1_000_000_000, 200);
        assert_eq!(entries, 5_000_000);

        // Test with zero avg_line_length (uses default 200)
        let entries_default = IndexProgress::estimate_entries(1_000_000_000, 0);
        assert_eq!(entries_default, 5_000_000);

        // 14GB file
        let entries_14gb = IndexProgress::estimate_entries(14_000_000_000, 200);
        assert_eq!(entries_14gb, 70_000_000);
    }

    /// Story 6.3 AC2: Verify serde camelCase serialization for all new fields
    #[test]
    fn test_serde_camel_case_serialization() {
        let progress = IndexProgress::with_batch_info(
            500_000_000,
            1_000_000_000,
            60.0,
            5_000_000,
            10_000_000,
            true,
            3,
            10,
        );

        let json = serde_json::to_string(&progress).expect("serialization failed");

        // Verify camelCase keys
        assert!(json.contains("\"percentage\""));
        assert!(json.contains("\"bytesProcessed\""));
        assert!(json.contains("\"totalBytes\""));
        assert!(json.contains("\"speedGbps\""));
        assert!(json.contains("\"etaSeconds\""));
        assert!(json.contains("\"entriesIndexed\""));
        assert!(json.contains("\"totalEntriesEstimated\""));
        assert!(json.contains("\"partialFilterAvailable\""));
        assert!(json.contains("\"batchesCompleted\""));
        assert!(json.contains("\"totalBatches\""));

        // Verify NO snake_case keys
        assert!(!json.contains("\"bytes_processed\""));
        assert!(!json.contains("\"total_bytes\""));
        assert!(!json.contains("\"entries_indexed\""));
        assert!(!json.contains("\"total_entries_estimated\""));
        assert!(!json.contains("\"partial_filter_available\""));
        assert!(!json.contains("\"batches_completed\""));
        assert!(!json.contains("\"total_batches\""));
    }

    /// Story 6.3 AC2: Test deserialization from JSON
    #[test]
    fn test_serde_deserialization() {
        let json = r#"{
            "percentage": 50.0,
            "bytesProcessed": 500000000,
            "totalBytes": 1000000000,
            "speedGbps": 0.5,
            "etaSeconds": 60.0,
            "entriesIndexed": 5000000,
            "totalEntriesEstimated": 10000000,
            "partialFilterAvailable": true,
            "batchesCompleted": 3,
            "totalBatches": 10
        }"#;

        let progress: IndexProgress = serde_json::from_str(json).expect("deserialization failed");

        assert!((progress.percentage - 50.0).abs() < 0.01);
        assert_eq!(progress.bytes_processed, 500_000_000);
        assert_eq!(progress.entries_indexed, 5_000_000);
        assert_eq!(progress.total_entries_estimated, 10_000_000);
        assert!(progress.partial_filter_available);
        assert_eq!(progress.batches_completed, 3);
        assert_eq!(progress.total_batches, 10);
    }
}
