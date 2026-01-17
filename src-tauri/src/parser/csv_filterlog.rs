use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use crate::types::log_entry::{LogEntry, LogFormat, ParseError, ParseResult};

// OPNsense CSV filterlog field indices (may vary by version)
const IDX_TIMESTAMP: usize = 0;
const IDX_INTERFACE: usize = 4;
const IDX_ACTION: usize = 6;
const IDX_SOURCE_IP: usize = 8;
const IDX_SOURCE_PORT: usize = 9;
const IDX_DEST_IP: usize = 10;
const IDX_DEST_PORT: usize = 11;
const IDX_PROTOCOL: usize = 16;
const IDX_RULE_LABEL: usize = 17;

pub fn parse_csv_filterlog_entry(line: &str, line_number: u64, entry_id: u64) -> ParseResult<LogEntry> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line.as_bytes());

    let record = reader.records().next()
        .ok_or_else(|| ParseError::MalformedCSV {
            line: line_number,
            reason: "Empty CSV line".to_string(),
        })?
        .map_err(|e| ParseError::MalformedCSV {
            line: line_number,
            reason: format!("CSV parsing error: {}", e),
        })?;

    // Ensure minimum field count
    if record.len() < 10 {
        return Err(ParseError::MalformedCSV {
            line: line_number,
            reason: format!("Insufficient fields: expected at least 10, got {}", record.len()),
        });
    }

    // Parse timestamp (Unix timestamp or ISO format)
    let timestamp_str = record.get(IDX_TIMESTAMP)
        .ok_or_else(|| ParseError::MalformedCSV {
            line: line_number,
            reason: "Missing timestamp field".to_string(),
        })?;

    let timestamp = parse_csv_timestamp(timestamp_str, line_number)?;

    // Extract fields with bounds checking
    let interface = safe_get(&record, IDX_INTERFACE);
    let action = safe_get(&record, IDX_ACTION);
    let source_ip = safe_get(&record, IDX_SOURCE_IP);
    let source_port = safe_get(&record, IDX_SOURCE_PORT).and_then(|s| s.parse().ok());
    let dest_ip = safe_get(&record, IDX_DEST_IP);
    let dest_port = safe_get(&record, IDX_DEST_PORT).and_then(|s| s.parse().ok());
    let protocol = safe_get(&record, IDX_PROTOCOL);
    let rule_label = safe_get(&record, IDX_RULE_LABEL);

    Ok(LogEntry {
        id: entry_id,
        format: LogFormat::CSV,
        timestamp,
        priority: None,
        facility: None,
        severity: None,
        hostname: None,
        process_name: None,
        process_id: None,
        message_id: None,
        structured_data: None,
        interface,
        source_ip,
        source_port,
        dest_ip,
        dest_port,
        protocol,
        action,
        rule_label,
        raw_line: line.to_string(),
        message: String::new(), // CSV format doesn't have a message field
    })
}

fn safe_get(record: &csv::StringRecord, index: usize) -> Option<String> {
    record.get(index)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn parse_csv_timestamp(timestamp_str: &str, line_number: u64) -> ParseResult<DateTime<Utc>> {
    // Try Unix timestamp first
    if let Ok(unix_ts) = timestamp_str.parse::<i64>() {
        return DateTime::<Utc>::from_timestamp(unix_ts, 0)
            .ok_or_else(|| ParseError::MalformedCSV {
                line: line_number,
                reason: format!("Invalid Unix timestamp: {}", unix_ts),
            });
    }

    // Try ISO 8601 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try custom OPNsense format (if different)
    // TODO: Add additional timestamp formats if needed

    Err(ParseError::MalformedCSV {
        line: line_number,
        reason: format!("Unrecognized timestamp format: {}", timestamp_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_csv_filterlog() {
        // CSV format with proper field indices matching IDX_ constants
        let line = "1705329000,111,222,333,vtnet0,555,pass,inet7,192.168.1.100,443,10.0.0.5,54321,13,14,15,16,TCP,abc123";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.interface.as_deref(), Some("vtnet0"));
        assert_eq!(entry.action.as_deref(), Some("pass"));
        assert_eq!(entry.source_ip.as_deref(), Some("192.168.1.100"));
        assert_eq!(entry.source_port, Some(443));
        assert_eq!(entry.dest_ip.as_deref(), Some("10.0.0.5"));
        assert_eq!(entry.dest_port, Some(54321));
        assert_eq!(entry.protocol.as_deref(), Some("TCP"));
        assert_eq!(entry.rule_label.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_malformed_csv_insufficient_fields() {
        let line = "timestamp,field1,field2";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_err());

        match result {
            Err(ParseError::MalformedCSV { line, reason }) => {
                assert_eq!(line, 1);
                assert!(reason.contains("Insufficient fields"));
            }
            _ => panic!("Expected MalformedCSV error"),
        }
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn csv_parser_never_panics(s in ".*") {
            // Parser should never panic, even on random input
            let result = parse_csv_filterlog_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn csv_parser_handles_quoted_fields(
            s in r#"[[:print:]]{0,500}"#
        ) {
            let result = parse_csv_filterlog_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }
    }
}
