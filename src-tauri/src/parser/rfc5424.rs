use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashMap;
use crate::types::log_entry::{LogEntry, LogFormat, ParseError, ParseResult};

lazy_static::lazy_static! {
    static ref RFC5424_REGEX: Regex = Regex::new(
        r"^<(?P<pri>\d{1,3})>(?P<ver>\d+)\s+(?P<timestamp>\S+)\s+(?P<hostname>\S+)\s+(?P<app>\S+)\s+(?P<procid>\S+)\s+(?P<msgid>\S+)\s+(?P<structured>\S+)\s*(?P<message>.*)$"
    ).unwrap();
}

pub fn parse_rfc5424_entry(line: &str, line_number: u64, entry_id: u64) -> ParseResult<LogEntry> {
    let captures = RFC5424_REGEX.captures(line)
        .ok_or_else(|| ParseError::MalformedRFC5424 {
            line: line_number,
            reason: "Line does not match RFC5424 pattern".to_string(),
        })?;

    // Parse priority
    let priority: u8 = captures.name("pri")
        .and_then(|m| m.as_str().parse().ok())
        .ok_or_else(|| ParseError::MalformedRFC5424 {
            line: line_number,
            reason: "Invalid priority field".to_string(),
        })?;

    let facility = priority / 8;
    let severity = priority % 8;

    // Parse ISO 8601 timestamp
    let timestamp_str = captures.name("timestamp")
        .map(|m| m.as_str())
        .ok_or_else(|| ParseError::MalformedRFC5424 {
            line: line_number,
            reason: "Missing timestamp".to_string(),
        })?;

    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ParseError::MalformedRFC5424 {
            line: line_number,
            reason: format!("Invalid ISO 8601 timestamp: {}", e),
        })?;

    // Extract other fields
    let hostname = nilvalue_to_option(captures.name("hostname").map(|m| m.as_str()));
    let process_name = nilvalue_to_option(captures.name("app").map(|m| m.as_str()));
    let process_id_str = nilvalue_to_option(captures.name("procid").map(|m| m.as_str()));
    let message_id = nilvalue_to_option(captures.name("msgid").map(|m| m.as_str()));

    let process_id = process_id_str.and_then(|s| s.parse().ok());

    // Parse structured data (simplified - real implementation more complex)
    let structured_str = captures.name("structured").map(|m| m.as_str()).unwrap_or("-");
    let structured_data = if structured_str != "-" {
        Some(parse_structured_data(structured_str))
    } else {
        None
    };

    let message = captures.name("message")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    Ok(LogEntry {
        id: entry_id,
        format: LogFormat::RFC5424,
        timestamp,
        priority: Some(priority),
        facility: Some(facility),
        severity: Some(severity),
        hostname,
        process_name,
        process_id,
        message_id,
        structured_data,
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

fn nilvalue_to_option(value: Option<&str>) -> Option<String> {
    value.and_then(|s| {
        if s == "-" {
            None
        } else {
            Some(s.to_string())
        }
    })
}

fn parse_structured_data(_data: &str) -> HashMap<String, String> {
    // RFC5424 structured data parsing
    // Format: [id key1="value1" key2="value2"] [id2 key3="value3"]
    // Simplified implementation: RFC5424 structured data is complex with escaped quotes
    // Full implementation would parse SD-IDs and SD-PARAMs with proper escaping
    // For OPNsense logs, structured data is typically "-" (nilvalue) or minimal
    // This simplified version returns empty map for nilvalue or unparsed data
    // Can be enhanced in future stories if structured data becomes critical

    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_rfc5424() {
        let line = "<134>1 2026-01-15T14:30:00.123Z firewall filterlog 12345 - - pass in on vtnet0";
        let result = parse_rfc5424_entry(line, 1, 1);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.priority, Some(134));
        assert_eq!(entry.hostname.as_deref(), Some("firewall"));
        assert_eq!(entry.process_name.as_deref(), Some("filterlog"));
    }

    #[test]
    fn test_nilvalue_fields_converted_to_none() {
        let line = "<134>1 2026-01-15T14:30:00Z - - - - - message";
        let result = parse_rfc5424_entry(line, 1, 1);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.hostname, None);
        assert_eq!(entry.process_name, None);
        assert_eq!(entry.message_id, None);
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn rfc5424_parser_never_panics(s in ".*") {
            // Parser should never panic, even on random input
            let result = parse_rfc5424_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn rfc5424_parser_handles_special_chars(
            s in r"[[:print:]]{0,1000}"
        ) {
            let result = parse_rfc5424_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }
    }
}
