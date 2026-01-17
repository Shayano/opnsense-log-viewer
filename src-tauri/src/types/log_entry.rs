use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Unified log entry structure for all formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Entry ID (assigned during indexing)
    pub id: u64,

    /// Detected or specified format
    pub format: LogFormat,

    /// Parsed timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Syslog priority (RFC3164/RFC5424 only)
    pub priority: Option<u8>,

    /// Facility (calculated from priority)
    pub facility: Option<u8>,

    /// Severity (calculated from priority)
    pub severity: Option<u8>,

    /// Hostname or IP
    pub hostname: Option<String>,

    /// Process name
    pub process_name: Option<String>,

    /// Process ID
    pub process_id: Option<u32>,

    /// Message ID (RFC5424 only)
    pub message_id: Option<String>,

    /// Structured data (RFC5424 only)
    pub structured_data: Option<HashMap<String, String>>,

    /// Interface (CSV filterlog)
    pub interface: Option<String>,

    /// Source IP address
    pub source_ip: Option<String>,

    /// Source port
    pub source_port: Option<u16>,

    /// Destination IP address
    pub dest_ip: Option<String>,

    /// Destination port
    pub dest_port: Option<u16>,

    /// Protocol (TCP, UDP, ICMP, etc.)
    pub protocol: Option<String>,

    /// Action (pass, block, reject)
    pub action: Option<String>,

    /// Rule hash or label
    pub rule_label: Option<String>,

    /// Raw log line
    pub raw_line: String,

    /// Message content
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogFormat {
    RFC3164,
    RFC5424,
    CSV,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseStatistics {
    pub total_lines: u64,
    pub parsed_successfully: u64,
    pub skipped_malformed: u64,
    pub format: LogFormat,
    pub errors: Vec<ParseErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseErrorInfo {
    pub line_number: u64,
    pub error_message: String,
    pub raw_line: String,
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Malformed RFC3164 entry at line {line}: {reason}")]
    MalformedRFC3164 { line: u64, reason: String },

    #[error("Malformed RFC5424 entry at line {line}: {reason}")]
    MalformedRFC5424 { line: u64, reason: String },

    #[error("Malformed CSV entry at line {line}: {reason}")]
    MalformedCSV { line: u64, reason: String },

    #[error("Unknown format - unable to detect from first {lines_sampled} lines")]
    UnknownFormat { lines_sampled: u64 },

    #[error("IO error: {0}")]
    IoError(String),

    #[error("UTF-8 decoding error at line {line}")]
    EncodingError { line: u64 },
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err.to_string())
    }
}
