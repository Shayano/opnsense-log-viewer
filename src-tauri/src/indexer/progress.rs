use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub percentage: f64,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub speed_gbps: f64,
    pub eta_seconds: f64,
}

impl IndexProgress {
    pub fn new(bytes_processed: u64, total_bytes: u64, elapsed_seconds: f64) -> Self {
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
        }
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
    }

    #[test]
    fn test_progress_completion() {
        let progress = IndexProgress::new(1024 * 1024 * 1024, 1024 * 1024 * 1024, 120.0);

        assert!((progress.percentage - 100.0).abs() < 0.1);
        assert_eq!(progress.bytes_processed, progress.total_bytes);
    }
}
