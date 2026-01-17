use chrono::{DateTime, Utc, NaiveDateTime, Datelike};
use regex::Regex;
use crate::types::log_entry::{LogEntry, LogFormat, ParseError, ParseResult};

lazy_static::lazy_static! {
    static ref RFC3164_REGEX: Regex = Regex::new(
        r"^<(?P<pri>\d{1,3})>(?P<timestamp>[A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(?P<hostname>\S+)\s+(?P<process>\S+?)(\[(?P<pid>\d+)\])?:\s*(?P<message>.*)$"
    ).unwrap();
}

pub fn parse_rfc3164_entry(line: &str, line_number: u64, entry_id: u64) -> ParseResult<LogEntry> {
    let captures = RFC3164_REGEX.captures(line)
        .ok_or_else(|| ParseError::MalformedRFC3164 {
            line: line_number,
            reason: "Line does not match RFC3164 pattern".to_string(),
        })?;

    // Parse priority (facility + severity)
    let priority: u8 = captures.name("pri")
        .and_then(|m| m.as_str().parse().ok())
        .ok_or_else(|| ParseError::MalformedRFC3164 {
            line: line_number,
            reason: "Invalid priority field".to_string(),
        })?;

    let facility = priority / 8;
    let severity = priority % 8;

    // Parse timestamp (BSD syslog format: "MMM DD HH:MM:SS")
    // Note: RFC3164 does not include year, assume current year
    let timestamp_str = captures.name("timestamp")
        .map(|m| m.as_str())
        .ok_or_else(|| ParseError::MalformedRFC3164 {
            line: line_number,
            reason: "Missing timestamp".to_string(),
        })?;

    let timestamp = parse_bsd_timestamp(timestamp_str, line_number)?;

    // Extract other fields
    let hostname = captures.name("hostname")
        .map(|m| m.as_str().to_string());

    let process_name = captures.name("process")
        .map(|m| m.as_str().to_string());

    let process_id = captures.name("pid")
        .and_then(|m| m.as_str().parse().ok());

    let message = captures.name("message")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    Ok(LogEntry {
        id: entry_id,
        format: LogFormat::RFC3164,
        timestamp,
        priority: Some(priority),
        facility: Some(facility),
        severity: Some(severity),
        hostname,
        process_name,
        process_id,
        message_id: None,
        structured_data: None,
        interface: None,
        source_ip: None,
        source_port: None,
        dest_ip: None,
        dest_port: None,
        protocol: None,
        action: None,
        rule_label: None,
        raw_line: line.to_string(),
        message,
    })
}

fn parse_bsd_timestamp(timestamp_str: &str, line_number: u64) -> ParseResult<DateTime<Utc>> {
    // BSD format: "Jan 15 14:30:00"
    // Since year is not included, assume current year
    let current_year = Utc::now().year();
    let full_timestamp = format!("{} {}", timestamp_str, current_year);

    NaiveDateTime::parse_from_str(&full_timestamp, "%b %d %H:%M:%S %Y")
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        .map_err(|e| ParseError::MalformedRFC3164 {
            line: line_number,
            reason: format!("Invalid timestamp format: {}", e),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_rfc3164() {
        let line = "<134>Jan 15 14:30:00 firewall filterlog[12345]: pass in on vtnet0";
        let result = parse_rfc3164_entry(line, 1, 1);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.priority, Some(134));
        assert_eq!(entry.facility, Some(16)); // 134 / 8 = 16
        assert_eq!(entry.severity, Some(6));  // 134 % 8 = 6
        assert_eq!(entry.hostname.as_deref(), Some("firewall"));
        assert_eq!(entry.process_name.as_deref(), Some("filterlog"));
        assert_eq!(entry.process_id, Some(12345));
    }

    #[test]
    fn test_parse_malformed_rfc3164_returns_error() {
        let line = "This is not a valid RFC3164 log entry";
        let result = parse_rfc3164_entry(line, 1, 1);
        assert!(result.is_err());

        match result {
            Err(ParseError::MalformedRFC3164 { line, .. }) => assert_eq!(line, 1),
            _ => panic!("Expected MalformedRFC3164 error"),
        }
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn rfc3164_parser_never_panics(s in ".*") {
            // Parser should never panic, even on random input
            let result = parse_rfc3164_entry(&s, 1, 1);
            // Either Ok or Err, but no panic
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn rfc3164_parser_handles_special_chars(
            s in r"[[:print:]]{0,1000}"
        ) {
            // Test with printable characters
            let result = parse_rfc3164_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }
    }
}
