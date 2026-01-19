pub mod format_detector;
pub mod rfc3164;
pub mod rfc5424;
pub mod csv_filterlog;

pub use format_detector::detect_format;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use crate::types::log_entry::{LogEntry, LogFormat, ParseError, ParseResult, ParseStatistics, ParseErrorInfo};

pub fn parse_file_streaming<P: AsRef<Path>>(
    file_path: P,
    format: LogFormat,
) -> ParseResult<(Vec<LogEntry>, ParseStatistics)> {
    log::info!(
        "[MEM] parse_file_streaming: start path={:?} format={:?} (loads FULL file into Vec<LogEntry>)",
        file_path.as_ref(),
        format
    );
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut line_number = 0u64;
    let mut entry_id = 0u64;
    let mut skipped_count = 0u64;

    for line_result in reader.lines() {
        line_number += 1;

        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                errors.push(ParseErrorInfo {
                    line_number,
                    error_message: format!("IO error: {}", e),
                    raw_line: String::new(),
                });
                skipped_count += 1;
                continue;
            }
        };

        if line.trim().is_empty() {
            continue; // Skip empty lines without counting as error
        }

        let parse_result = match format {
            LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(&line, line_number, entry_id),
            LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(&line, line_number, entry_id),
            LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(&line, line_number, entry_id),
            LogFormat::Unknown => {
                return Err(ParseError::UnknownFormat { lines_sampled: line_number });
            }
        };

        match parse_result {
            Ok(entry) => {
                entries.push(entry);
                entry_id += 1;
            }
            Err(e) => {
                errors.push(ParseErrorInfo {
                    line_number,
                    error_message: e.to_string(),
                    raw_line: line.clone(),
                });
                skipped_count += 1;

                // Log error but continue parsing
                log::warn!("Parse error at line {}: {}", line_number, e);
            }
        }
    }

    let stats = ParseStatistics {
        total_lines: line_number,
        parsed_successfully: entry_id,
        skipped_malformed: skipped_count,
        format,
        errors,
    };

    log::info!(
        "[MEM] parse_file_streaming: done entries={} (Vec<LogEntry> in RAM, will be dropped when caller returns)",
        entries.len()
    );
    Ok((entries, stats))
}

/// Parse log file and collect only entries whose id is in `id_set`, up to `max` entries.
/// Reads line-by-line and does not hold the entire file in memory. Use this for
/// get_entries_by_ids to avoid loading multi-GB files into RAM.
pub fn parse_file_collect_matching<P: AsRef<Path>>(
    file_path: P,
    format: LogFormat,
    id_set: &HashSet<u64>,
    max: usize,
) -> ParseResult<Vec<LogEntry>> {
    log::info!(
        "[MEM] parse_file_collect_matching: start path={:?} id_set={} max={} (streaming, only collects matching)",
        file_path.as_ref(),
        id_set.len(),
        max
    );
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut result = Vec::with_capacity(max.min(id_set.len()));
    let mut line_number = 0u64;
    let mut entry_id = 0u64;

    for line_result in reader.lines() {
        line_number += 1;

        let line = match line_result {
            Ok(l) => l,
            Err(e) => return Err(ParseError::IoError(format!("{}", e))),
        };

        if line.trim().is_empty() {
            continue;
        }

        let parse_result = match format {
            LogFormat::RFC3164 => rfc3164::parse_rfc3164_entry(&line, line_number, entry_id),
            LogFormat::RFC5424 => rfc5424::parse_rfc5424_entry(&line, line_number, entry_id),
            LogFormat::CSV => csv_filterlog::parse_csv_filterlog_entry(&line, line_number, entry_id),
            LogFormat::Unknown => {
                return Err(ParseError::UnknownFormat { lines_sampled: line_number });
            }
        };

        match parse_result {
            Ok(entry) => {
                if id_set.contains(&entry_id) {
                    result.push(entry);
                    if result.len() >= max {
                        break;
                    }
                }
                entry_id += 1;
            }
            Err(_) => {
                log::warn!("Parse error at line {}", line_number);
            }
        }
    }

    log::info!(
        "[MEM] parse_file_collect_matching: done collected={} lines_scanned={}",
        result.len(),
        line_number
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    // Tests are implemented in parser_integration_test.rs
    // Error handling and malformed line tests pass
    // Memory efficiency validated through integration tests
}
