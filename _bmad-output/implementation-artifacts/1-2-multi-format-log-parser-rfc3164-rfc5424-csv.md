# Story 1.2: Multi-Format Log Parser (RFC3164, RFC5424, CSV)

Status: ready-for-dev

## Story

As a network administrator,
I want the application to automatically detect and parse OPNsense log formats,
So that I don't need to manually specify the log format for each file.

## Acceptance Criteria

**Given** a log file is opened
**When** the parser analyzes the first 100 lines
**Then** it detects the format as one of: RFC3164, RFC5424, or CSV filterlog

**When** the format is RFC3164 (legacy syslog)
**Then** entries are parsed extracting:
- Priority (PRI)
- Timestamp
- Hostname
- Process name and PID
- Message content

**When** the format is RFC5424 (modern syslog)
**Then** entries are parsed extracting:
- Priority, Version, Timestamp
- Hostname, App-name, Process ID
- Message ID, Structured data
- Message content

**When** the format is CSV filterlog (OPNsense specific)
**Then** entries are parsed extracting all comma-separated fields:
- Timestamp, Interface, Action (pass/block/reject)
- Source IP, Source Port
- Destination IP, Destination Port
- Protocol, Rule hash/label

**When** the format is ambiguous or unrecognized
**Then** a dialog prompts: "Select log format: [RFC3164] [RFC5424] [CSV filterlog]"
**And** the user's selection is applied to parse the file

**When** malformed entries are encountered during parsing
**Then** the parser:
- Logs the line number and error to application log
- Skips the malformed entry
- Continues parsing subsequent entries
- Reports total skipped count after completion: "Parsed X entries, skipped Y malformed lines"

**And** parsing accuracy meets NFR-002.2:
- 100% accuracy for well-formed entries
- <0.1% discrepancy vs Python tool validation

## Tasks / Subtasks

- [ ] Create parser module structure (AC: Module organization)
  - [ ] Create src-tauri/src/parser/mod.rs with public API
  - [ ] Create src-tauri/src/parser/format_detector.rs
  - [ ] Create src-tauri/src/parser/rfc3164.rs
  - [ ] Create src-tauri/src/parser/rfc5424.rs
  - [ ] Create src-tauri/src/parser/csv_filterlog.rs
  - [ ] Export DetectedFormat enum and parse_entry() function

- [ ] Implement LogEntry and ParsedEntry types (AC: Type definitions)
  - [ ] Create src-tauri/src/types/log_entry.rs
  - [ ] Define LogEntry struct with all parsed fields
  - [ ] Define ParseResult<T> = Result<T, ParseError>
  - [ ] Define ParseError enum with specific error variants
  - [ ] Add serde Serialize/Deserialize for IPC serialization

- [ ] Implement format detection (AC: Auto-detect log format)
  - [ ] Read first 100 lines of file using BufReader
  - [ ] Attempt to match each line against RFC3164 regex pattern
  - [ ] Attempt to match each line against RFC5424 regex pattern
  - [ ] Attempt to match each line against CSV filterlog pattern
  - [ ] Return DetectedFormat::RFC3164/RFC5424/CSV based on highest confidence match
  - [ ] Return DetectedFormat::Unknown if ambiguous (<80% match on any format)
  - [ ] Handle empty files and files with <100 lines gracefully

- [ ] Implement RFC3164 parser (AC: Parse legacy syslog)
  - [ ] Define regex pattern for RFC3164: `<PRI>TIMESTAMP HOSTNAME PROCESS[PID]: MESSAGE`
  - [ ] Extract priority field (calculate facility and severity)
  - [ ] Parse timestamp (various BSD syslog formats: MMM DD HH:MM:SS)
  - [ ] Extract hostname field
  - [ ] Extract process name and PID from bracketed format
  - [ ] Extract message content
  - [ ] Return ParseError::MalformedRFC3164 for invalid lines
  - [ ] Add unit tests with valid and malformed samples

- [ ] Implement RFC5424 parser (AC: Parse modern syslog)
  - [ ] Define regex pattern for RFC5424: `<PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG`
  - [ ] Extract priority and version fields
  - [ ] Parse ISO 8601 timestamp with timezone
  - [ ] Extract hostname, app-name, process ID, message ID
  - [ ] Parse structured data (key-value pairs in brackets)
  - [ ] Extract message content
  - [ ] Return ParseError::MalformedRFC5424 for invalid lines
  - [ ] Add unit tests with valid and malformed samples

- [ ] Implement CSV filterlog parser (AC: Parse OPNsense CSV)
  - [ ] Parse CSV line with proper escaping (handle quoted fields with commas)
  - [ ] Map CSV column indices to LogEntry fields:
    - Column 0: Timestamp (Unix timestamp or ISO format)
    - Column 4: Interface (vtnet0, em0)
    - Column 6: Action (pass, block, reject)
    - Column 8: Source IP
    - Column 9: Source Port
    - Column 10: Destination IP
    - Column 11: Destination Port
    - Column 16: Protocol (TCP, UDP, ICMP)
    - Column 17: Rule hash/label
  - [ ] Handle variable CSV column counts (OPNsense formats vary by version)
  - [ ] Return ParseError::MalformedCSV for invalid lines
  - [ ] Add unit tests with valid and malformed samples

- [ ] Implement streaming parser with error recovery (AC: Handle malformed entries)
  - [ ] Create parse_file_streaming() function with BufReader
  - [ ] Track line number for error reporting
  - [ ] Collect ParseError instances with line numbers
  - [ ] Continue parsing after errors (no panics)
  - [ ] Return ParseResult with successful entries + error summary
  - [ ] Report statistics: total lines, parsed, skipped, errors
  - [ ] Ensure memory usage stays <500 MB (streaming, no full file load)

- [ ] Create format selection dialog (AC: Manual format selection)
  - [ ] Create src/components/format-selector/format-selector.tsx
  - [ ] Display modal when format is DetectedFormat::Unknown
  - [ ] Show three radio buttons: RFC3164, RFC5424, CSV filterlog
  - [ ] Include brief description for each format with example
  - [ ] Add [Confirm] and [Cancel] buttons
  - [ ] Call parse_file_with_format() IPC command with user selection
  - [ ] Add unit tests for component

- [ ] Update index_file command to use parser (AC: Integration)
  - [ ] Modify src-tauri/src/commands/indexation.rs
  - [ ] Call format_detector::detect_format() on file
  - [ ] If format is Unknown, return error with prompt for manual selection
  - [ ] Call appropriate parser based on DetectedFormat
  - [ ] Collect parsed LogEntry instances
  - [ ] Update IndexMetadata with detected format
  - [ ] Handle parsing errors gracefully, log to application log
  - [ ] Return parsing statistics in response

- [ ] Add Tauri IPC command for manual format selection (AC: User override)
  - [ ] Create parse_file_with_format(file_path: String, format: String) command
  - [ ] Validate format parameter (must be "RFC3164", "RFC5424", or "CSV")
  - [ ] Call appropriate parser directly without detection
  - [ ] Return IndexMetadata with user-selected format

- [ ] Write property-based tests for parser robustness (AC: NFR-002.2)
  - [ ] Use proptest crate to generate random log entries
  - [ ] Fuzz parsers with malformed data (empty lines, truncated, special chars)
  - [ ] Run 1000+ iterations per test case
  - [ ] Ensure parsers never panic, always return ParseError for invalid input
  - [ ] Validate 100% accuracy on well-formed entries

- [ ] Write integration tests (AC: Parsing accuracy)
  - [ ] Create test fixtures with real OPNsense log samples
  - [ ] Parse fixtures with each parser (RFC3164, RFC5424, CSV)
  - [ ] Compare parsed results with expected ground truth
  - [ ] Validate <0.1% discrepancy vs Python tool (if available)
  - [ ] Test format detection accuracy (95%+ on mixed files)

- [ ] Write unit tests for all parsers (AC: Tests pass)
  - [ ] Test RFC3164 parser with valid BSD syslog entries
  - [ ] Test RFC5424 parser with valid modern syslog entries
  - [ ] Test CSV filterlog parser with OPNsense CSV samples
  - [ ] Test malformed entry handling (errors returned, not panics)
  - [ ] Test edge cases: empty files, single-line files, Unicode characters
  - [ ] Ensure all tests pass with `cargo test`

## Dev Notes

### 🔥 ULTIMATE STORY CONTEXT ENGINE OUTPUT 🔥

This story file has been generated by the comprehensive context analysis engine. It contains EVERYTHING you need to implement Story 1.2 correctly, aligned with architecture, UX design, and project patterns.

---

### Architecture Compliance & Technical Requirements

#### **Technology Stack (MUST USE EXACT VERSIONS)**

**Backend (Rust):**
- Regex parsing: regex = "1.10" (for pattern matching RFC3164/RFC5424)
- CSV parsing: csv = "1.3" (for OPNsense filterlog CSV)
- Date/time parsing: chrono = "0.4.39" (already added in Story 1.1)
- Error handling: thiserror = "2.0" (for ParseError enum)
- Async I/O: tokio = { version = "1.49", features = ["rt-multi-thread", "fs", "io-util"] } (already added)
- Property-based testing: proptest = "1.4" (for fuzzing parsers - dev dependency)

**Frontend (TypeScript):**
- Format selector modal: Use base Modal component from Story 0.3
- No new dependencies required for this story

**Testing:**
- Rust unit tests: Built-in `#[cfg(test)]` modules
- Property-based tests: proptest crate
- Integration tests: Real log file fixtures in tests/fixtures/

---

#### **Code Structure & File Organization**

**Backend Structure:**
```
src-tauri/src/
├── parser/
│   ├── mod.rs                      # Public API: detect_format(), parse_entry()
│   ├── format_detector.rs          # Auto-detection logic (first 100 lines)
│   ├── rfc3164.rs                  # BSD Syslog parser (legacy)
│   ├── rfc5424.rs                  # Modern Syslog parser
│   └── csv_filterlog.rs            # OPNsense CSV parser
├── types/
│   ├── mod.rs                      # Re-exports
│   └── log_entry.rs                # LogEntry, ParseError, ParseResult
├── commands/
│   └── indexation.rs               # Updated to use parser (Story 1.1)
└── lib.rs                          # Register new IPC commands
```

**Frontend Structure:**
```
src/
└── components/
    └── format-selector/
        ├── format-selector.tsx     # Manual format selection modal
        ├── format-selector.test.tsx
        └── index.ts                # Barrel export
```

---

#### **Log Entry Type Definition**

**File: src-tauri/src/types/log_entry.rs**

```rust
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
```

---

#### **Format Detection Logic**

**File: src-tauri/src/parser/format_detector.rs**

```rust
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
    use super::*;

    #[test]
    fn test_detect_rfc3164_format() {
        // Test with sample RFC3164 data
        // TODO: Create test fixture file
    }

    #[test]
    fn test_detect_rfc5424_format() {
        // Test with sample RFC5424 data
    }

    #[test]
    fn test_detect_csv_format() {
        // Test with OPNsense CSV data
    }

    #[test]
    fn test_ambiguous_format_returns_unknown() {
        // Test with mixed format file
    }
}
```

---

#### **RFC3164 Parser Implementation**

**File: src-tauri/src/parser/rfc3164.rs**

```rust
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
```

---

#### **RFC5424 Parser Implementation**

**File: src-tauri/src/parser/rfc5424.rs**

```rust
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

fn parse_structured_data(data: &str) -> HashMap<String, String> {
    // Simplified implementation - real RFC5424 structured data is more complex
    // Format: [id key1="value1" key2="value2"]
    let mut map = HashMap::new();

    // TODO: Implement proper RFC5424 structured data parsing
    // For now, return empty map

    map
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
```

---

#### **CSV Filterlog Parser Implementation**

**File: src-tauri/src/parser/csv_filterlog.rs**

```rust
use chrono::{DateTime, Utc, NaiveDateTime};
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
        return Ok(DateTime::<Utc>::from_timestamp(unix_ts, 0)
            .ok_or_else(|| ParseError::MalformedCSV {
                line: line_number,
                reason: format!("Invalid Unix timestamp: {}", unix_ts),
            })?);
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
        let line = "1705329000,,,vtnet0,,pass,inet,192.168.1.100,443,10.0.0.5,54321,,,TCP,,,abc123";
        let result = parse_csv_filterlog_entry(line, 1, 1);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.interface.as_deref(), Some("vtnet0"));
        assert_eq!(entry.action.as_deref(), Some("pass"));
        assert_eq!(entry.source_ip.as_deref(), Some("192.168.1.100"));
        assert_eq!(entry.source_port, Some(443));
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
```

---

#### **Streaming Parser with Error Recovery**

**File: src-tauri/src/parser/mod.rs**

```rust
mod format_detector;
mod rfc3164;
mod rfc5424;
mod csv_filterlog;

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
    use super::*;

    #[test]
    fn test_parse_file_streaming_handles_errors() {
        // Create test file with mixed valid and invalid lines
        // TODO: Implement with tempfile
    }

    #[test]
    fn test_parse_file_streaming_memory_efficient() {
        // Verify memory usage stays below threshold
        // TODO: Implement memory profiling test
    }
}
```

---

### Library & Framework Requirements

#### **New Dependencies for Cargo.toml:**

```toml
[dependencies]
regex = "1.10"
csv = "1.3"
lazy_static = "1.4"
log = "0.4"

[dev-dependencies]
proptest = "1.4"
tempfile = "3.14"
```

#### **Existing Dependencies (from Story 1.1):**
- chrono = "0.4.39" (already added)
- thiserror = "2.0" (add if not present)
- tokio = { version = "1.49", features = ["rt-multi-thread", "fs", "io-util"] }

---

### Testing Requirements

#### **Property-Based Tests (Fuzzing)**

**File: src-tauri/src/parser/tests/proptest_parsers.rs**

```rust
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn rfc3164_parser_never_panics(s in ".*") {
            // Parser should never panic, even on random input
            let result = rfc3164::parse_rfc3164_entry(&s, 1, 1);
            // Either Ok or Err, but no panic
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn rfc5424_parser_never_panics(s in ".*") {
            let result = rfc5424::parse_rfc5424_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }

        #[test]
        fn csv_parser_never_panics(s in ".*") {
            let result = csv_filterlog::parse_csv_filterlog_entry(&s, 1, 1);
            assert!(result.is_ok() || result.is_err());
        }
    }
}
```

#### **Integration Tests with Real Data**

**File: tests/parser_integration_test.rs**

```rust
use std::fs;

#[test]
fn test_parse_real_opnsense_logs() {
    // Load real OPNsense log samples from tests/fixtures/
    let rfc3164_sample = fs::read_to_string("tests/fixtures/opnsense_rfc3164.log")
        .expect("RFC3164 fixture missing");

    let rfc5424_sample = fs::read_to_string("tests/fixtures/opnsense_rfc5424.log")
        .expect("RFC5424 fixture missing");

    let csv_sample = fs::read_to_string("tests/fixtures/opnsense_filterlog.csv")
        .expect("CSV fixture missing");

    // Test format detection
    // Test parsing accuracy
    // Validate 100% success on well-formed entries
}
```

---

### Security Requirements

#### **Input Validation:**
- NO panic on malformed input (must return ParseError)
- NO buffer overflows (Rust memory safety)
- NO infinite loops (tested with fuzzing)

#### **Resource Limits:**
- Streaming parsing prevents full file load into memory
- BufReader with default buffer size (8 KB)
- Memory usage must stay <500 MB even for 30 GB files

---

### UX Design Requirements

#### **Format Selection Dialog**

**Component: src/components/format-selector/format-selector.tsx**

```typescript
import { useId } from 'react';
import { Modal } from '@/components/base/modal';
import { Button } from '@/components/base/button';

interface FormatSelectorProps {
  isOpen: boolean;
  onConfirm: (format: 'RFC3164' | 'RFC5424' | 'CSV') => void;
  onCancel: () => void;
}

export function FormatSelector({ isOpen, onConfirm, onCancel }: FormatSelectorProps) {
  const [selectedFormat, setSelectedFormat] = useState<'RFC3164' | 'RFC5424' | 'CSV'>('RFC3164');
  const radioGroupId = useId();

  return (
    <Modal isOpen={isOpen} onClose={onCancel} title="Select Log Format">
      <div className="space-y-4">
        <p className="text-sm text-gray-600 dark:text-gray-400">
          Unable to auto-detect log format. Please select manually:
        </p>

        <div className="space-y-2" role="radiogroup" aria-labelledby={radioGroupId}>
          <label className="flex items-start space-x-3 p-3 border rounded hover:bg-gray-50 dark:hover:bg-gray-800">
            <input
              type="radio"
              name="format"
              value="RFC3164"
              checked={selectedFormat === 'RFC3164'}
              onChange={() => setSelectedFormat('RFC3164')}
              className="mt-1"
            />
            <div>
              <div className="font-medium">RFC3164 (Legacy Syslog)</div>
              <div className="text-sm text-gray-500">
                Example: &lt;134&gt;Jan 15 14:30:00 firewall filterlog[123]: message
              </div>
            </div>
          </label>

          <label className="flex items-start space-x-3 p-3 border rounded hover:bg-gray-50 dark:hover:bg-gray-800">
            <input
              type="radio"
              name="format"
              value="RFC5424"
              checked={selectedFormat === 'RFC5424'}
              onChange={() => setSelectedFormat('RFC5424')}
              className="mt-1"
            />
            <div>
              <div className="font-medium">RFC5424 (Modern Syslog)</div>
              <div className="text-sm text-gray-500">
                Example: &lt;134&gt;1 2026-01-15T14:30:00Z firewall filterlog 123 - - message
              </div>
            </div>
          </label>

          <label className="flex items-start space-x-3 p-3 border rounded hover:bg-gray-50 dark:hover:bg-gray-800">
            <input
              type="radio"
              name="format"
              value="CSV"
              checked={selectedFormat === 'CSV'}
              onChange={() => setSelectedFormat('CSV')}
              className="mt-1"
            />
            <div>
              <div className="font-medium">CSV filterlog (OPNsense)</div>
              <div className="text-sm text-gray-500">
                Example: 1705329000,,,vtnet0,,pass,inet,192.168.1.100,443,...
              </div>
            </div>
          </label>
        </div>

        <div className="flex justify-end space-x-2 pt-4">
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => onConfirm(selectedFormat)}>
            Confirm
          </Button>
        </div>
      </div>
    </Modal>
  );
}
```

---

### Previous Story Intelligence

#### **Learnings from Story 1.1 (File Selection):**

**What Works Well:**
- Tauri IPC command pattern with Result<T, String> error handling
- Zustand store pattern for state management
- BufReader for streaming file operations
- tempfile crate for testing with temporary files

**Code Patterns to Reuse:**
- IPC command registration in src-tauri/src/lib.rs
- Error handling with thiserror custom error types
- #[serde(rename_all = "camelCase")] for TypeScript interop
- React 18 patterns: useId(), no React imports

**Files to Extend:**
- src-tauri/src/commands/indexation.rs (add parser integration)
- src-tauri/src/types/mod.rs (add log_entry module)
- src/stores/file-store.ts (add parsing statistics)

---

### Git Intelligence Summary

**Recent Work Pattern:**
- Story 1.1 just completed (file selection with native OS picker)
- All tests passing (41 frontend + 28 backend)
- Code review completed with adversarial review finding 10 issues
- 3 HIGH and 4 MEDIUM issues fixed, 3 LOW remain (non-blocking)

**Dependencies Available:**
- chrono = "0.4.39" (already added for timestamp handling)
- tokio = "1.49.0" with async features
- tempfile = "3.14.0" (for testing)
- serde with derive features

**Next Integration Point:**
- Story 1.2 extends index_file command from Story 1.1
- Parser will be called by indexation command
- Format detection happens before indexing
- Parsed entries will be used by Story 1.3 (Hybrid Index Creation)

---

### Latest Technical Research

#### **Regex Crate Best Practices (2026):**
- Use `lazy_static` for regex compilation (compile once, reuse)
- Named capture groups improve readability: `(?P<name>...)`
- Use `Regex::is_match()` for detection, `captures()` for extraction
- Set byte limits for untrusted input to prevent ReDoS attacks

#### **CSV Crate Best Practices:**
- Use `ReaderBuilder::new().has_headers(false)` for headerless CSV
- Always check field bounds with `record.get(index)`
- Handle quoted fields automatically (CSV crate does this)
- Use `StringRecord::get()` instead of indexing to avoid panics

#### **Chrono Date Parsing:**
- RFC3164 timestamps lack year, assume current year
- Use `DateTime::parse_from_rfc3339()` for ISO 8601
- Unix timestamps: `DateTime::from_timestamp()`
- Always use `Utc` timezone for consistency

#### **OPNsense Log Format Variations:**
- CSV filterlog field count varies by OPNsense version
- Some versions use Unix timestamps, others ISO 8601
- Rule labels can be hashes (abc123) or descriptions
- Interface names are physical (vtnet0) until enrichment (Story 3.x)

---

### Project Context Reference

**Critical Rules from project-context.md:**

**Error Handling:**
- Rust: NEVER panic in production code
- Use `Result<T, ParseError>` for fallible operations
- Use `thiserror` for custom error types with context
- Log errors with `log::warn!()` but continue processing

**Memory Safety:**
- Use BufReader for streaming large files
- NEVER load full file into memory with `read_to_string()`
- Allocate Vec with capacity if size known in advance
- Clear collections after processing to free memory

**Testing:**
- Property-based testing with `proptest` for parser robustness
- Integration tests with real log file samples
- Unit tests for all parsers (valid and malformed inputs)
- Use `tempfile` for temporary test files

**Performance:**
- Regex compiled once with `lazy_static`
- Avoid unnecessary allocations (use `&str` not `String`)
- Use `BufReader` with 8 KB buffer (default)
- Benchmark parsing speed: target <7 sec/GB

---

### Story Completion Status

**Status:** ready-for-dev

**Next Steps:**
1. Review this story file thoroughly - contains ALL context needed
2. Implement tasks/subtasks in order (types → detector → parsers → integration → tests)
3. Follow architecture patterns and project context rules
4. Run tests after each module: `cargo test`
5. Run property-based tests: `cargo test --release` (proptest)
6. Test format detection with real OPNsense logs
7. Commit with format: `Complete Story 1.2: Multi-Format Log Parser (RFC3164, RFC5424, CSV)`
8. Run code-review workflow after completion

**Estimated Complexity:** High (8-12 hours)
- Type definitions and error types: 1-2 hours
- Format detector: 1 hour
- RFC3164 parser: 2-3 hours (timestamp parsing tricky)
- RFC5424 parser: 2-3 hours (structured data complex)
- CSV filterlog parser: 1-2 hours
- Streaming parser with error recovery: 1-2 hours
- Property-based tests + integration tests: 2-3 hours

**Blocking Dependencies:**
- Story 1.1 (File Selection with Native OS Picker) ✅ DONE

**Blocked Stories:**
- Story 1.3 (Hybrid Index Creation) - requires parsed LogEntry instances
- Story 1.4 (Index Persistence & Reuse) - requires LogEntry serialization
- Story 1.5 (Log Entry Display Table) - requires LogEntry for display

---

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)

### Debug Log References

N/A - Story created via create-story workflow

### Completion Notes List

Story file created by BMad Method create-story workflow with comprehensive context analysis.

**Key Implementation Notes:**
- Parser MUST be memory-efficient (streaming, no full file load)
- Parser MUST NEVER panic (return ParseError instead)
- Format detection uses first 100 lines, 80% confidence threshold
- Malformed entries are skipped with error logging, not fatal
- Property-based testing with `proptest` ensures robustness
- Integration with Story 1.1's index_file command

**Architecture Compliance:**
- Uses regex crate for pattern matching (architectural decision)
- Uses csv crate for CSV parsing (architectural decision)
- Uses chrono for timestamp parsing (already added in Story 1.1)
- Uses thiserror for error types (architectural decision)
- Memory usage <500 MB (NFR-001.4)

**Testing Requirements:**
- Unit tests for each parser (valid + malformed inputs)
- Property-based tests with proptest (fuzzing)
- Integration tests with real OPNsense log samples
- Parsing accuracy validation: 100% for well-formed, <0.1% discrepancy vs Python

### File List

**Files to Create:**
- src-tauri/src/parser/mod.rs
- src-tauri/src/parser/format_detector.rs
- src-tauri/src/parser/rfc3164.rs
- src-tauri/src/parser/rfc5424.rs
- src-tauri/src/parser/csv_filterlog.rs
- src-tauri/src/types/log_entry.rs
- src-tauri/src/parser/tests/proptest_parsers.rs
- tests/parser_integration_test.rs
- tests/fixtures/opnsense_rfc3164.log (sample data)
- tests/fixtures/opnsense_rfc5424.log (sample data)
- tests/fixtures/opnsense_filterlog.csv (sample data)
- src/components/format-selector/format-selector.tsx
- src/components/format-selector/format-selector.test.tsx
- src/components/format-selector/index.ts

**Files to Modify:**
- src-tauri/src/commands/indexation.rs (integrate parser)
- src-tauri/src/lib.rs (register parse_file_with_format command)
- src-tauri/src/types/mod.rs (add log_entry module)
- src-tauri/Cargo.toml (add regex, csv, lazy_static dependencies)
- src/stores/file-store.ts (add parseStatistics field)
