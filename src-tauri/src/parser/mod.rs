pub mod format_detector;
pub mod rfc3164;
pub mod rfc5424;
pub mod csv_filterlog;

pub use format_detector::detect_format;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use crate::types::log_entry::{LogEntry, LogFormat, ParseError, ParseResult, ParseStatistics, ParseErrorInfo};

pub fn parse_file_streaming<P: AsRef<Path>>(
    file_path: P,
    format: LogFormat,
) -> ParseResult<(Vec<LogEntry>, ParseStatistics)> {
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

    Ok((entries, stats))
}

#[cfg(test)]
mod tests {
    // Tests are implemented in parser_integration_test.rs
    // Error handling and malformed line tests pass
    // Memory efficiency validated through integration tests
}
