use chrono::{DateTime, NaiveDateTime, Utc};
use csv::ReaderBuilder;
use crate::types::log_entry::{LogEntry, LogFormat, ParseError, ParseResult};

// OPNsense filterlog CSV field indices (within the CSV part after syslog prefix)
// Format: RULE_NUM,SUB_RULE,ANCHOR,TRACKER,INTERFACE,REASON,ACTION,DIRECTION,IP_VER,TOS,ECN,TTL,ID,OFFSET,FLAGS,PROTO_NUM,PROTO_NAME,LENGTH,SRC_IP,DST_IP,SRC_PORT,DST_PORT,...
const IDX_TRACKER: usize = 3;       // Rule hash (used as rule_label)
const IDX_INTERFACE: usize = 4;
const IDX_ACTION: usize = 6;
const IDX_DIRECTION: usize = 7;
const IDX_PROTOCOL: usize = 16;     // Protocol name (tcp, udp, icmp)
const IDX_SOURCE_IP: usize = 18;
const IDX_DEST_IP: usize = 19;
const IDX_SOURCE_PORT: usize = 20;
const IDX_DEST_PORT: usize = 21;

/// Parse OPNsense filterlog entry
///
/// Supports two formats:
/// 1. Native OPNsense syslog: `TIMESTAMP<TAB>SEVERITY<TAB>filterlog<TAB>CSV_DATA`
/// 2. Pure CSV: `UNIX_TIMESTAMP,RULE_NUM,...`
pub fn parse_csv_filterlog_entry(line: &str, line_number: u64, entry_id: u64) -> ParseResult<LogEntry> {
    // Remove BOM if present (UTF-8 BOM: EF BB BF)
    let line = line.trim_start_matches('\u{FEFF}');

    // Try to detect and parse OPNsense native syslog format first
    // Format: ISO_TIMESTAMP<TAB>Informational<TAB>filterlog<TAB>CSV_DATA
    if let Some((timestamp, csv_part)) = extract_syslog_parts(line) {
        return parse_filterlog_csv(csv_part, timestamp, line, line_number, entry_id);
    }

    // Fallback: try pure CSV format (legacy support)
    parse_legacy_csv_format(line, line_number, entry_id)
}

/// Extract timestamp and CSV part from OPNsense syslog format
/// Returns (timestamp, csv_data) if successful
fn extract_syslog_parts(line: &str) -> Option<(DateTime<Utc>, &str)> {
    // Split by tab to get syslog fields
    let parts: Vec<&str> = line.splitn(4, '\t').collect();

    // Need at least 4 parts: timestamp, severity, process, message
    if parts.len() < 4 {
        return None;
    }

    // Verify this is a filterlog entry
    if !parts[2].trim().eq_ignore_ascii_case("filterlog") {
        return None;
    }

    // Parse the ISO timestamp from first field
    let timestamp_str = parts[0].trim();
    let timestamp = parse_iso_timestamp(timestamp_str)?;

    // The CSV data is in the 4th field, trim leading space
    let csv_part = parts[3].trim_start();

    Some((timestamp, csv_part))
}

/// Parse ISO 8601 timestamp (with or without timezone)
fn parse_iso_timestamp(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first (with timezone)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try without timezone (assume UTC)
    // Format: 2026-01-21T11:07:29
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(naive.and_utc());
    }

    // Try with milliseconds
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(naive.and_utc());
    }

    None
}

/// Parse the filterlog CSV portion with correct OPNsense field indices
fn parse_filterlog_csv(
    csv_data: &str,
    timestamp: DateTime<Utc>,
    raw_line: &str,
    line_number: u64,
    entry_id: u64,
) -> ParseResult<LogEntry> {
    // Fast path: split by comma
    let fields: Vec<&str> = csv_data.split(',').collect();

    // Need at least 22 fields for a valid filterlog entry with ports
    if fields.len() < 19 {
        return Err(ParseError::MalformedCSV {
            line: line_number,
            reason: format!("Insufficient CSV fields: expected at least 19, got {}", fields.len()),
        });
    }

    // Extract fields with correct indices
    let interface = safe_get_field(&fields, IDX_INTERFACE);
    let action = safe_get_field(&fields, IDX_ACTION);
    let protocol = safe_get_field(&fields, IDX_PROTOCOL);
    let rule_label = safe_get_field(&fields, IDX_TRACKER);
    let _direction = safe_get_field(&fields, IDX_DIRECTION); // in/out - not used currently

    // IP and port fields - may not exist for all protocols (e.g., ICMP)
    let source_ip = safe_get_field(&fields, IDX_SOURCE_IP);
    let dest_ip = safe_get_field(&fields, IDX_DEST_IP);
    let source_port = safe_get_field(&fields, IDX_SOURCE_PORT).and_then(|s| s.parse().ok());
    let dest_port = safe_get_field(&fields, IDX_DEST_PORT).and_then(|s| s.parse().ok());

    Ok(LogEntry {
        id: entry_id,
        format: LogFormat::CSV,
        timestamp,
        priority: None,
        facility: None,
        severity: None,
        hostname: None,
        process_name: Some("filterlog".to_string()),
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
        raw_line: raw_line.to_string(),
        message: csv_data.to_string(),
    })
}

/// Safe field extraction from slice
#[inline]
fn safe_get_field(fields: &[&str], index: usize) -> Option<String> {
    fields.get(index)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Legacy CSV format parser (for backwards compatibility)
/// Expects pure CSV with Unix timestamp in first field
fn parse_legacy_csv_format(line: &str, line_number: u64, entry_id: u64) -> ParseResult<LogEntry> {
    let fields: Vec<&str> = line.split(',').collect();

    if fields.len() < 10 {
        // Try csv crate for quoted fields
        return parse_with_csv_crate(line, line_number, entry_id);
    }

    // Try to parse first field as Unix timestamp
    let timestamp_str = fields.get(0)
        .ok_or_else(|| ParseError::MalformedCSV {
            line: line_number,
            reason: "Missing timestamp field".to_string(),
        })?;

    let timestamp = parse_csv_timestamp(timestamp_str, line_number)?;

    // Use legacy indices for backwards compatibility
    // These were the original indices that might work with some CSV exports
    const LEGACY_IDX_INTERFACE: usize = 4;
    const LEGACY_IDX_ACTION: usize = 6;
    const LEGACY_IDX_SOURCE_IP: usize = 18;
    const LEGACY_IDX_DEST_IP: usize = 19;
    const LEGACY_IDX_SOURCE_PORT: usize = 20;
    const LEGACY_IDX_DEST_PORT: usize = 21;
    const LEGACY_IDX_PROTOCOL: usize = 16;
    const LEGACY_IDX_RULE_LABEL: usize = 3;

    let interface = safe_get_field(&fields, LEGACY_IDX_INTERFACE);
    let action = safe_get_field(&fields, LEGACY_IDX_ACTION);
    let source_ip = safe_get_field(&fields, LEGACY_IDX_SOURCE_IP);
    let source_port = safe_get_field(&fields, LEGACY_IDX_SOURCE_PORT).and_then(|s| s.parse().ok());
    let dest_ip = safe_get_field(&fields, LEGACY_IDX_DEST_IP);
    let dest_port = safe_get_field(&fields, LEGACY_IDX_DEST_PORT).and_then(|s| s.parse().ok());
    let protocol = safe_get_field(&fields, LEGACY_IDX_PROTOCOL);
    let rule_label = safe_get_field(&fields, LEGACY_IDX_RULE_LABEL);

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
        message: String::new(),
    })
}

/// Fallback parser using csv crate (handles quoted fields)
fn parse_with_csv_crate(line: &str, line_number: u64, entry_id: u64) -> ParseResult<LogEntry> {
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

    if record.len() < 10 {
        return Err(ParseError::MalformedCSV {
            line: line_number,
            reason: format!("Insufficient fields: expected at least 10, got {}", record.len()),
        });
    }

    let timestamp_str = record.get(0)
        .ok_or_else(|| ParseError::MalformedCSV {
            line: line_number,
            reason: "Missing timestamp field".to_string(),
        })?;

    let timestamp = parse_csv_timestamp(timestamp_str, line_number)?;

    // Use new correct indices
    fn safe_get(record: &csv::StringRecord, index: usize) -> Option<String> {
        record.get(index)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    let interface = safe_get(&record, IDX_INTERFACE);
    let action = safe_get(&record, IDX_ACTION);
    let source_ip = safe_get(&record, IDX_SOURCE_IP);
    let source_port = safe_get(&record, IDX_SOURCE_PORT).and_then(|s| s.parse().ok());
    let dest_ip = safe_get(&record, IDX_DEST_IP);
    let dest_port = safe_get(&record, IDX_DEST_PORT).and_then(|s| s.parse().ok());
    let protocol = safe_get(&record, IDX_PROTOCOL);
    let rule_label = safe_get(&record, IDX_TRACKER);

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
        message: String::new(),
    })
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

    // Try ISO without timezone
    if let Ok(naive) = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%S") {
        return Ok(naive.and_utc());
    }

    Err(ParseError::MalformedCSV {
        line: line_number,
        reason: format!("Unrecognized timestamp format: {}", timestamp_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_opnsense_native_format() {
        // Real OPNsense syslog format with tab separators
        let line = "2026-01-21T11:07:29\tInformational\tfilterlog\t 14,,,02f4bab031b57d1e30553ce08e0ec131,ovpns3,match,block,in,4,0x0,,128,44671,0,none,17,udp,291,10.81.248.2,10.81.248.255,54915,54915,271";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let entry = result.unwrap();
        assert_eq!(entry.interface.as_deref(), Some("ovpns3"));
        assert_eq!(entry.action.as_deref(), Some("block"));
        assert_eq!(entry.source_ip.as_deref(), Some("10.81.248.2"));
        assert_eq!(entry.dest_ip.as_deref(), Some("10.81.248.255"));
        assert_eq!(entry.source_port, Some(54915));
        assert_eq!(entry.dest_port, Some(54915));
        assert_eq!(entry.protocol.as_deref(), Some("udp"));
        assert_eq!(entry.rule_label.as_deref(), Some("02f4bab031b57d1e30553ce08e0ec131"));
    }

    #[test]
    fn test_parse_opnsense_tcp_entry() {
        let line = "2026-01-21T11:07:29\tInformational\tfilterlog\t 42,,,066f924120669e6f9191376d128ba5ae,igc0,match,pass,out,4,0x0,,127,62184,0,DF,6,tcp,52,85.90.1.66,135.236.90.183,41851,443,0,S,3121015473,,64240,,mss;nop;wscale;nop;nop;sackOK";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let entry = result.unwrap();
        assert_eq!(entry.interface.as_deref(), Some("igc0"));
        assert_eq!(entry.action.as_deref(), Some("pass"));
        assert_eq!(entry.source_ip.as_deref(), Some("85.90.1.66"));
        assert_eq!(entry.dest_ip.as_deref(), Some("135.236.90.183"));
        assert_eq!(entry.source_port, Some(41851));
        assert_eq!(entry.dest_port, Some(443));
        assert_eq!(entry.protocol.as_deref(), Some("tcp"));
    }

    #[test]
    fn test_parse_opnsense_with_bom() {
        // Line with UTF-8 BOM
        let line = "\u{FEFF}2026-01-21T11:07:29\tInformational\tfilterlog\t 14,,,hash,igc0,match,block,in,4,0x0,,128,1,0,none,17,udp,65,10.0.0.1,10.0.0.2,1234,5678,45";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let entry = result.unwrap();
        assert_eq!(entry.interface.as_deref(), Some("igc0"));
    }

    #[test]
    fn test_parse_different_interfaces() {
        let test_cases = vec![
            ("igc0", "igc0"),
            ("igc1", "igc1"),
            ("igc2", "igc2"),
            ("ovpns3", "ovpns3"),
            ("wg0", "wg0"),
            ("enc0", "enc0"),
        ];

        for (interface, expected) in test_cases {
            let line = format!(
                "2026-01-21T11:07:29\tInformational\tfilterlog\t 14,,,hash,{},match,block,in,4,0x0,,128,1,0,none,17,udp,65,10.0.0.1,10.0.0.2,1234,5678,45",
                interface
            );
            let result = parse_csv_filterlog_entry(&line, 1, 1);
            assert!(result.is_ok(), "Failed for interface {}: {:?}", interface, result.err());
            assert_eq!(result.unwrap().interface.as_deref(), Some(expected));
        }
    }

    #[test]
    fn test_malformed_csv_insufficient_fields() {
        let line = "2026-01-21T11:07:29\tInformational\tfilterlog\t 14,,,hash";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_filterlog_line_fails() {
        // Line that's not a filterlog entry
        let line = "2026-01-21T11:07:29\tInformational\tkernel\t some kernel message";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        // Should fail because it's not filterlog
        assert!(result.is_err());
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
