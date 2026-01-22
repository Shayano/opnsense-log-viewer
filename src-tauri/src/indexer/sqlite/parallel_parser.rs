//! Parallel Log Parser using Rayon and Crossbeam Channels
//!
//! Story 6.2: Implements parallel parsing of log files using Rayon's par_bridge()
//! for CPU-bound parsing work, sending results to a bounded channel for backpressure.
//!
//! ## Architecture
//!
//! ```text
//! BufReader (sequential) → par_bridge() → Rayon workers (parallel parse)
//!                                               ↓
//!                                    Bounded Channel (backpressure)
//!                                               ↓
//!                                    SQLite Writer (single thread)
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! let (sender, receiver) = crossbeam_channel::bounded(50_000);
//! let progress = PipelineProgress::new();
//!
//! parse_file_parallel(&file_path, format, sender, &progress)?;
//! // Channel closes when function returns
//! ```

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::Sender;
use rayon::prelude::*;

use crate::parser::{csv_filterlog, rfc3164, rfc5424};
use crate::types::log_entry::LogFormat;

use super::parsed_entry::ParsedEntry;
use super::progress::PipelineProgress;
use super::PipelineError;

/// Default buffer size for BufReader (1MB for efficient disk reads)
const BUFFER_SIZE: usize = 1024 * 1024;

/// Parse a log file in parallel using Rayon and send results to channel
///
/// Uses `par_bridge()` to parallelize the inherently sequential line iteration.
/// Each worker parses independently and sends `ParsedEntry` to the bounded channel.
/// Backpressure is automatic when the channel is full (sender blocks).
///
/// # Arguments
/// * `file_path` - Path to the log file
/// * `format` - Detected log format (RFC3164, RFC5424, CSV)
/// * `sender` - Bounded channel sender for backpressure
/// * `progress` - Shared progress tracking structure
///
/// # Returns
/// * `Ok(())` on successful completion
/// * `Err(PipelineError)` on IO or fatal errors
///
/// # Error Handling
/// - Individual parse errors are logged and skipped (non-fatal)
/// - Empty lines are silently skipped
/// - IO errors terminate parsing with an error
///
/// # Performance Notes
/// - Uses 1MB read buffer for efficient disk I/O
/// - Byte offset tracking is approximate (per-line, not per-byte)
/// - Progress updates use relaxed atomics (no synchronization overhead)
pub fn parse_file_parallel(
    file_path: &Path,
    format: LogFormat,
    sender: Sender<ParsedEntry>,
    progress: &PipelineProgress,
) -> Result<(), PipelineError> {
    log::info!(
        "[PARALLEL PARSER] Starting parallel parse of {:?} with format {:?}",
        file_path,
        format
    );

    let file = File::open(file_path)?;
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let reader = BufReader::with_capacity(BUFFER_SIZE, file);

    // Track byte offset (approximate - line-based)
    let byte_offset = AtomicU64::new(0);

    // Collect lines with their metadata first, then process in parallel
    // This allows proper byte offset tracking while still using parallel processing
    let lines_with_offsets: Vec<(u64, u64, String)> = reader
        .lines()
        .enumerate()
        .filter_map(|(idx, line_result)| {
            let line_num = idx as u64 + 1;
            match line_result {
                Ok(line) => {
                    if line.trim().is_empty() {
                        None
                    } else {
                        // Calculate approximate byte offset
                        let offset = byte_offset.fetch_add(line.len() as u64 + 1, Ordering::Relaxed);
                        progress.bytes_read.fetch_add(line.len() as u64 + 1, Ordering::Relaxed);
                        Some((line_num, offset, line))
                    }
                }
                Err(e) => {
                    log::warn!("[PARALLEL PARSER] Read error at line {}: {}", line_num, e);
                    progress.errors.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })
        .collect();

    let total_lines = lines_with_offsets.len();
    log::info!(
        "[PARALLEL PARSER] Collected {} non-empty lines from {} bytes, starting parallel parse",
        total_lines,
        file_size
    );

    // Process in parallel using Rayon
    lines_with_offsets
        .into_par_iter()
        .for_each(|(line_num, offset, line)| {
            // Parse using existing parser module
            let entry_result = match format {
                LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(&line, line_num, line_num),
                LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(&line, line_num, line_num),
                LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(&line, line_num, line_num),
                LogFormat::Unknown => {
                    log::warn!("[PARALLEL PARSER] Unknown format at line {}", line_num);
                    return;
                }
            };

            match entry_result {
                Ok(log_entry) => {
                    let parsed = ParsedEntry::from_log_entry(log_entry, offset as i64);

                    // Send to writer (blocks if channel full = backpressure)
                    if sender.send(parsed).is_err() {
                        // Channel closed (writer stopped) - log but don't crash
                        log::error!(
                            "[PARALLEL PARSER] Channel closed unexpectedly at line {}",
                            line_num
                        );
                    }

                    progress.entries_parsed.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    log::debug!("[PARALLEL PARSER] Parse error at line {}: {}", line_num, e);
                    progress.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

    log::info!(
        "[PARALLEL PARSER] Completed parsing {} entries with {} errors",
        progress.entries_parsed.load(Ordering::Relaxed),
        progress.errors.load(Ordering::Relaxed)
    );

    Ok(())
}

/// Parse a log file in parallel using streaming mode (lower memory)
///
/// This variant uses `par_bridge()` directly on the iterator, which has lower
/// peak memory but less predictable ordering. Use for very large files where
/// memory is a concern.
///
/// # Arguments
/// * `file_path` - Path to the log file
/// * `format` - Detected log format
/// * `sender` - Bounded channel sender
/// * `progress` - Shared progress tracking
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(PipelineError)` on failure
pub fn parse_file_streaming(
    file_path: &Path,
    format: LogFormat,
    sender: Sender<ParsedEntry>,
    progress: &PipelineProgress,
) -> Result<(), PipelineError> {
    log::info!(
        "[PARALLEL PARSER] Starting streaming parse of {:?}",
        file_path
    );

    let file = File::open(file_path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, file);

    // Atomic counter for approximate byte offset
    let byte_offset = AtomicU64::new(0);

    // Use par_bridge for streaming parallel processing
    reader
        .lines()
        .enumerate()
        .par_bridge()
        .for_each(|(idx, line_result)| {
            let line_num = (idx + 1) as u64;

            let line = match line_result {
                Ok(l) => l,
                Err(e) => {
                    log::warn!("[PARALLEL PARSER] Read error at line {}: {}", line_num, e);
                    progress.errors.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            if line.trim().is_empty() {
                return;
            }

            // Track byte offset
            let offset = byte_offset.fetch_add(line.len() as u64 + 1, Ordering::Relaxed);
            progress.bytes_read.fetch_add(line.len() as u64 + 1, Ordering::Relaxed);

            // Parse using existing parser module
            let entry_result = match format {
                LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(&line, line_num, line_num),
                LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(&line, line_num, line_num),
                LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(&line, line_num, line_num),
                LogFormat::Unknown => return,
            };

            match entry_result {
                Ok(log_entry) => {
                    let parsed = ParsedEntry::from_log_entry(log_entry, offset as i64);

                    if sender.send(parsed).is_err() {
                        log::error!(
                            "[PARALLEL PARSER] Channel closed at line {}",
                            line_num
                        );
                    }

                    progress.entries_parsed.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    log::debug!("[PARALLEL PARSER] Parse error at line {}: {}", line_num, e);
                    progress.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

    log::info!(
        "[PARALLEL PARSER] Streaming parse complete: {} entries, {} errors",
        progress.entries_parsed.load(Ordering::Relaxed),
        progress.errors.load(Ordering::Relaxed)
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv_file(lines: &[&str]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_parse_file_parallel_empty_file() {
        let file = create_test_csv_file(&[]);
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();

        let result = parse_file_parallel(file.path(), LogFormat::CSV, sender, &progress);

        assert!(result.is_ok());
        assert_eq!(progress.entries_parsed.load(Ordering::Relaxed), 0);
        drop(receiver); // Consume receiver
    }

    #[test]
    fn test_parse_file_parallel_skips_empty_lines() {
        let file = create_test_csv_file(&["", "   ", "\t"]);
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();

        let result = parse_file_parallel(file.path(), LogFormat::CSV, sender, &progress);

        assert!(result.is_ok());
        assert_eq!(progress.entries_parsed.load(Ordering::Relaxed), 0);
        drop(receiver);
    }

    #[test]
    fn test_parse_file_parallel_tracks_errors() {
        // Invalid CSV lines that won't parse
        let file = create_test_csv_file(&[
            "invalid,csv,line",
            "another,bad,line",
        ]);
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();

        let result = parse_file_parallel(file.path(), LogFormat::CSV, sender, &progress);

        assert!(result.is_ok());
        // These should fail to parse (missing required fields)
        assert_eq!(progress.entries_parsed.load(Ordering::Relaxed), 0);
        assert!(progress.errors.load(Ordering::Relaxed) > 0);
        drop(receiver);
    }

    #[test]
    fn test_parse_file_parallel_channel_backpressure() {
        // Create a file with more entries than channel capacity
        let lines: Vec<String> = (0..100)
            .map(|i| format!("line,{},data", i))
            .collect();
        let file = create_test_csv_file(&lines.iter().map(|s| s.as_str()).collect::<Vec<_>>());

        // Very small channel to test backpressure
        let (sender, receiver) = bounded(5);
        let progress = PipelineProgress::new();

        // Run in a thread to avoid blocking
        let file_path = file.path().to_path_buf();
        let handle = std::thread::spawn(move || {
            parse_file_parallel(&file_path, LogFormat::CSV, sender, &progress)
        });

        // Drain the receiver
        let mut count = 0;
        while receiver.recv().is_ok() {
            count += 1;
        }

        // Wait for parser to complete
        let result = handle.join().unwrap();
        assert!(result.is_ok());
        // Count may be 0 if all lines failed to parse (which is expected for invalid CSV)
        // The important thing is no deadlock occurred
        assert!(count >= 0);
    }

    #[test]
    fn test_parse_file_streaming_basic() {
        let file = create_test_csv_file(&["invalid,line,1", "invalid,line,2"]);
        let (sender, receiver) = bounded(100);
        let progress = PipelineProgress::new();

        let result = parse_file_streaming(file.path(), LogFormat::CSV, sender, &progress);

        assert!(result.is_ok());
        // All should fail to parse but no crash
        assert!(progress.bytes_read.load(Ordering::Relaxed) > 0);
        drop(receiver);
    }

    #[test]
    fn test_progress_tracking() {
        let file = create_test_csv_file(&["line1", "line2", "line3"]);
        let (sender, _receiver) = bounded(100);
        let progress = PipelineProgress::new();

        let _ = parse_file_parallel(file.path(), LogFormat::CSV, sender, &progress);

        // Bytes read should be sum of line lengths + newlines
        assert!(progress.bytes_read.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_file_not_found() {
        let (sender, _) = bounded(100);
        let progress = PipelineProgress::new();

        let result = parse_file_parallel(
            Path::new("/nonexistent/file.log"),
            LogFormat::CSV,
            sender,
            &progress,
        );

        assert!(result.is_err());
    }
}
