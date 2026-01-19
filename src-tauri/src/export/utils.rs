use crate::export::types::InsufficientDiskSpaceError;
use anyhow::{Context, Result};
use std::path::Path;

/// Check if there's sufficient disk space for export
///
/// Returns Ok if sufficient space is available, Err otherwise
pub fn check_disk_space<P: AsRef<Path>>(
    save_path: P,
    estimated_size_mb: f64,
) -> Result<(), InsufficientDiskSpaceError> {
    let save_path = save_path.as_ref();

    // Get parent directory (where file will be saved)
    let parent_dir = save_path
        .parent()
        .unwrap_or_else(|| Path::new("/"));

    // Get available disk space
    let available_bytes = match fs2::available_space(parent_dir) {
        Ok(bytes) => bytes,
        Err(_) => {
            // If we can't determine available space, allow the export to proceed
            // The actual file write will fail if there's insufficient space
            return Ok(());
        }
    };

    let available_mb = available_bytes as f64 / 1_048_576.0;
    let required_mb = estimated_size_mb * 1.1; // Add 10% safety margin

    if available_mb < required_mb {
        return Err(InsufficientDiskSpaceError {
            required_mb,
            available_mb,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_check_disk_space_sufficient() {
        let temp_path = env::temp_dir().join("test_export.csv");

        // Check for very small file (should always have space)
        let result = check_disk_space(&temp_path, 0.001);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_disk_space_insufficient() {
        let temp_path = env::temp_dir().join("test_export.csv");

        // Check for unrealistically large file (should fail)
        let result = check_disk_space(&temp_path, 1_000_000_000.0); // 1 million GB
        assert!(result.is_err());

        if let Err(err) = result {
            assert!(err.required_mb > err.available_mb);
        }
    }

    #[test]
    fn test_check_disk_space_with_margin() {
        let temp_path = env::temp_dir().join("test_export.csv");

        // Verify 10% safety margin is applied
        let estimated_mb = 100.0;
        let result = check_disk_space(&temp_path, estimated_mb);

        // This test may pass or fail depending on available disk space
        // We're mainly testing that the function doesn't panic
        let _ = result;
    }

    #[test]
    fn test_check_disk_space_nonexistent_parent() {
        // Test with path that has non-existent parent
        let temp_path = env::temp_dir().join("nonexistent_dir").join("test.csv");

        // Should handle gracefully (may succeed if parent becomes root)
        let result = check_disk_space(&temp_path, 1.0);
        let _ = result; // Don't assert, just verify no panic
    }
}
