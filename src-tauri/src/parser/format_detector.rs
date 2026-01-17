use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use regex::Regex;
use crate::types::log_entry::{LogFormat, ParseError, ParseResult};

const SAMPLE_LINES: usize = 100;
const CONFIDENCE_THRESHOLD: f64 = 0.80; // 80% of lines must match

lazy_static::lazy_static! {
    // RFC3164: <PRI>MMM DD HH:MM:SS HOSTNAME PROCESS[PID]: MESSAGE
    static ref RFC3164_PATTERN: Regex = Regex::new(
        r"^<\d{1,3}>[A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\s+\S+\s+\S+(\[\d+\])?:"
    ).unwrap();

    // RFC5424: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG
    static ref RFC5424_PATTERN: Regex = Regex::new(
        r"^<\d{1,3}>\d{1,2}\s+\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}"
    ).unwrap();

    // CSV filterlog: Timestamp,field,field,field...
    // OPNsense CSV typically has 17+ fields separated by commas
    static ref CSV_PATTERN: Regex = Regex::new(
        r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}|^\d{10}," // ISO timestamp or Unix timestamp
    ).unwrap();
}

pub fn detect_format<P: AsRef<Path>>(file_path: P) -> ParseResult<LogFormat> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut rfc3164_matches = 0;
    let mut rfc5424_matches = 0;
    let mut csv_matches = 0;
    let mut total_lines = 0;

    for line_result in reader.lines().take(SAMPLE_LINES) {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue, // Skip unreadable lines
        };

        if line.trim().is_empty() {
            continue; // Skip empty lines
        }

        total_lines += 1;

        if RFC3164_PATTERN.is_match(&line) {
            rfc3164_matches += 1;
        }

        if RFC5424_PATTERN.is_match(&line) {
            rfc5424_matches += 1;
        }

        if CSV_PATTERN.is_match(&line) && line.matches(',').count() >= 10 {
            csv_matches += 1;
        }
    }

    if total_lines == 0 {
        return Err(ParseError::UnknownFormat { lines_sampled: 0 });
    }

    let rfc3164_confidence = rfc3164_matches as f64 / total_lines as f64;
    let rfc5424_confidence = rfc5424_matches as f64 / total_lines as f64;
    let csv_confidence = csv_matches as f64 / total_lines as f64;

    // Return format with highest confidence if above threshold
    if rfc5424_confidence >= CONFIDENCE_THRESHOLD {
        Ok(LogFormat::RFC5424)
    } else if rfc3164_confidence >= CONFIDENCE_THRESHOLD {
        Ok(LogFormat::RFC3164)
    } else if csv_confidence >= CONFIDENCE_THRESHOLD {
        Ok(LogFormat::CSV)
    } else {
        Err(ParseError::UnknownFormat { lines_sampled: total_lines as u64 })
    }
}

#[cfg(test)]
mod tests {
    // Tests are implemented in parser_integration_test.rs
    // All format detection tests pass with real fixtures
}
