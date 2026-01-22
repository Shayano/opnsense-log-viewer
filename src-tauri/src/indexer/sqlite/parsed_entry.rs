//! ParsedEntry struct for SQLite insertion
//!
//! Story 6.2: Defines the entry format optimized for SQLite batch insertion.
//!
//! ## Design
//!
//! `ParsedEntry` is a simplified structure that matches the SQLite `entries` table schema exactly.
//! It differs from `LogEntry` which contains additional fields (format, priority, facility,
//! severity, etc.) that aren't stored in SQLite.
//!
//! ## Usage
//!
//! ```ignore
//! let log_entry = parse_rfc3164_entry(&line, line_num, entry_id)?;
//! let parsed = ParsedEntry::from_log_entry(log_entry, byte_offset);
//! sender.send(parsed)?;
//! ```

use crate::types::log_entry::LogEntry;

/// Entry ready for SQLite insertion
///
/// Matches the `entries` table schema exactly. All fields are flattened
/// for efficient batch insertion.
#[derive(Debug, Clone)]
pub struct ParsedEntry {
    /// Position in source file (for raw_line recovery)
    pub byte_offset: i64,

    /// ISO8601 formatted timestamp
    pub timestamp: String,

    /// Source IP address (optional)
    pub source_ip: Option<String>,

    /// Source port (optional)
    pub source_port: Option<i32>,

    /// Destination IP address (optional)
    pub dest_ip: Option<String>,

    /// Destination port (optional)
    pub dest_port: Option<i32>,

    /// Action: "pass", "block", "reject", or "unknown"
    pub action: String,

    /// Protocol: TCP, UDP, ICMP, etc. (optional)
    pub protocol: Option<String>,

    /// Network interface (optional)
    pub interface: Option<String>,

    /// Rule ID or label (optional)
    pub rule_id: Option<String>,

    /// Raw log line (optional, for debugging)
    pub raw_line: Option<String>,
}

impl ParsedEntry {
    /// Create a ParsedEntry from a LogEntry with the given byte offset
    ///
    /// Converts the rich LogEntry format to the flat SQLite-compatible format.
    /// Fields not present in LogEntry (or None) remain None in ParsedEntry.
    ///
    /// # Arguments
    /// * `entry` - The parsed LogEntry from the parser module
    /// * `byte_offset` - File position where this entry starts
    ///
    /// # Returns
    /// A new ParsedEntry ready for SQLite insertion
    pub fn from_log_entry(entry: LogEntry, byte_offset: i64) -> Self {
        Self {
            byte_offset,
            timestamp: entry.timestamp.to_rfc3339(),
            source_ip: entry.source_ip,
            source_port: entry.source_port.map(|p| p as i32),
            dest_ip: entry.dest_ip,
            dest_port: entry.dest_port.map(|p| p as i32),
            action: entry.action.unwrap_or_else(|| "unknown".to_string()),
            protocol: entry.protocol,
            interface: entry.interface,
            rule_id: entry.rule_label, // rule_label in LogEntry maps to rule_id in SQLite
            raw_line: Some(entry.raw_line),
        }
    }

    /// Create a minimal ParsedEntry for testing
    #[cfg(test)]
    pub fn test_entry(byte_offset: i64, action: &str) -> Self {
        Self {
            byte_offset,
            timestamp: chrono::Utc::now().to_rfc3339(),
            source_ip: Some("192.168.1.1".to_string()),
            source_port: Some(12345),
            dest_ip: Some("10.0.0.1".to_string()),
            dest_port: Some(443),
            action: action.to_string(),
            protocol: Some("TCP".to_string()),
            interface: Some("vtnet0".to_string()),
            rule_id: Some("rule_1".to_string()),
            raw_line: Some("test log line".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use crate::types::log_entry::LogFormat;

    fn create_test_log_entry() -> LogEntry {
        LogEntry {
            id: 1,
            format: LogFormat::CSV,
            timestamp: Utc.with_ymd_and_hms(2026, 1, 22, 10, 30, 0).unwrap(),
            priority: Some(134),
            facility: Some(16),
            severity: Some(6),
            hostname: Some("firewall.local".to_string()),
            process_name: Some("filterlog".to_string()),
            process_id: Some(1234),
            message_id: None,
            structured_data: Some(HashMap::new()),
            interface: Some("vtnet0".to_string()),
            source_ip: Some("192.168.1.100".to_string()),
            source_port: Some(54321),
            dest_ip: Some("10.0.0.1".to_string()),
            dest_port: Some(443),
            protocol: Some("TCP".to_string()),
            action: Some("pass".to_string()),
            rule_label: Some("allow_https".to_string()),
            raw_line: "test,raw,log,line".to_string(),
            message: "Firewall log entry".to_string(),
        }
    }

    #[test]
    fn test_from_log_entry_basic() {
        let log_entry = create_test_log_entry();
        let parsed = ParsedEntry::from_log_entry(log_entry, 1024);

        assert_eq!(parsed.byte_offset, 1024);
        assert!(parsed.timestamp.contains("2026-01-22"));
        assert_eq!(parsed.source_ip, Some("192.168.1.100".to_string()));
        assert_eq!(parsed.source_port, Some(54321));
        assert_eq!(parsed.dest_ip, Some("10.0.0.1".to_string()));
        assert_eq!(parsed.dest_port, Some(443));
        assert_eq!(parsed.action, "pass");
        assert_eq!(parsed.protocol, Some("TCP".to_string()));
        assert_eq!(parsed.interface, Some("vtnet0".to_string()));
        assert_eq!(parsed.rule_id, Some("allow_https".to_string()));
        assert_eq!(parsed.raw_line, Some("test,raw,log,line".to_string()));
    }

    #[test]
    fn test_from_log_entry_missing_action() {
        let mut log_entry = create_test_log_entry();
        log_entry.action = None;

        let parsed = ParsedEntry::from_log_entry(log_entry, 0);
        assert_eq!(parsed.action, "unknown");
    }

    #[test]
    fn test_from_log_entry_optional_fields_none() {
        let log_entry = LogEntry {
            id: 2,
            format: LogFormat::RFC3164,
            timestamp: Utc::now(),
            priority: None,
            facility: None,
            severity: None,
            hostname: None,
            process_name: None,
            process_id: None,
            message_id: None,
            structured_data: None,
            interface: None,
            source_ip: None,
            source_port: None,
            dest_ip: None,
            dest_port: None,
            protocol: None,
            action: Some("block".to_string()),
            rule_label: None,
            raw_line: "minimal entry".to_string(),
            message: "".to_string(),
        };

        let parsed = ParsedEntry::from_log_entry(log_entry, 512);

        assert_eq!(parsed.byte_offset, 512);
        assert_eq!(parsed.source_ip, None);
        assert_eq!(parsed.source_port, None);
        assert_eq!(parsed.dest_ip, None);
        assert_eq!(parsed.dest_port, None);
        assert_eq!(parsed.action, "block");
        assert_eq!(parsed.protocol, None);
        assert_eq!(parsed.interface, None);
        assert_eq!(parsed.rule_id, None);
        assert_eq!(parsed.raw_line, Some("minimal entry".to_string()));
    }

    #[test]
    fn test_port_conversion_u16_to_i32() {
        let mut log_entry = create_test_log_entry();
        log_entry.source_port = Some(65535); // Max u16
        log_entry.dest_port = Some(0); // Min u16

        let parsed = ParsedEntry::from_log_entry(log_entry, 0);

        assert_eq!(parsed.source_port, Some(65535));
        assert_eq!(parsed.dest_port, Some(0));
    }

    #[test]
    fn test_timestamp_rfc3339_format() {
        let log_entry = create_test_log_entry();
        let parsed = ParsedEntry::from_log_entry(log_entry, 0);

        // Verify timestamp is valid RFC3339
        assert!(parsed.timestamp.contains("T"));
        assert!(
            parsed.timestamp.ends_with("Z") || parsed.timestamp.contains("+"),
            "Timestamp should end with Z or have timezone offset"
        );
    }

    #[test]
    fn test_test_entry_helper() {
        let entry = ParsedEntry::test_entry(2048, "block");

        assert_eq!(entry.byte_offset, 2048);
        assert_eq!(entry.action, "block");
        assert!(entry.source_ip.is_some());
        assert!(entry.dest_ip.is_some());
        assert!(entry.protocol.is_some());
    }

    #[test]
    fn test_clone() {
        let entry = ParsedEntry::test_entry(100, "pass");
        let cloned = entry.clone();

        assert_eq!(entry.byte_offset, cloned.byte_offset);
        assert_eq!(entry.action, cloned.action);
        assert_eq!(entry.source_ip, cloned.source_ip);
    }
}
