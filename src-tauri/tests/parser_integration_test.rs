use opnsense_log_viewer_lib::parser::{detect_format, parse_file_streaming};
use opnsense_log_viewer_lib::types::log_entry::LogFormat;

#[test]
fn test_detect_rfc3164_format() {
    let fixture_path = "tests/fixtures/opnsense_rfc3164.log";
    let result = detect_format(fixture_path);

    assert!(result.is_ok(), "Format detection should succeed");
    assert_eq!(result.unwrap(), LogFormat::RFC3164);
}

#[test]
fn test_detect_rfc5424_format() {
    let fixture_path = "tests/fixtures/opnsense_rfc5424.log";
    let result = detect_format(fixture_path);

    assert!(result.is_ok(), "Format detection should succeed");
    assert_eq!(result.unwrap(), LogFormat::RFC5424);
}

#[test]
fn test_detect_csv_format() {
    let fixture_path = "tests/fixtures/opnsense_filterlog.csv";
    let result = detect_format(fixture_path);

    assert!(result.is_ok(), "Format detection should succeed");
    assert_eq!(result.unwrap(), LogFormat::CSV);
}

#[test]
fn test_parse_rfc3164_file() {
    let fixture_path = "tests/fixtures/opnsense_rfc3164.log";
    let result = parse_file_streaming(fixture_path, LogFormat::RFC3164);

    assert!(result.is_ok(), "Parsing should succeed");
    let (entries, stats) = result.unwrap();

    assert_eq!(entries.len(), 5, "Should parse all 5 entries");
    assert_eq!(stats.parsed_successfully, 5);
    assert_eq!(stats.skipped_malformed, 0);
    assert_eq!(stats.format, LogFormat::RFC3164);

    // Verify first entry details
    let first_entry = &entries[0];
    assert_eq!(first_entry.id, 0);
    assert_eq!(first_entry.format, LogFormat::RFC3164);
    assert_eq!(first_entry.priority, Some(134));
    assert_eq!(first_entry.facility, Some(16)); // 134 / 8 = 16
    assert_eq!(first_entry.severity, Some(6));  // 134 % 8 = 6
    assert_eq!(first_entry.hostname.as_deref(), Some("firewall"));
    assert_eq!(first_entry.process_name.as_deref(), Some("filterlog"));
    assert_eq!(first_entry.process_id, Some(12345));
}

#[test]
fn test_parse_rfc5424_file() {
    let fixture_path = "tests/fixtures/opnsense_rfc5424.log";
    let result = parse_file_streaming(fixture_path, LogFormat::RFC5424);

    assert!(result.is_ok(), "Parsing should succeed");
    let (entries, stats) = result.unwrap();

    assert_eq!(entries.len(), 5, "Should parse all 5 entries");
    assert_eq!(stats.parsed_successfully, 5);
    assert_eq!(stats.skipped_malformed, 0);
    assert_eq!(stats.format, LogFormat::RFC5424);

    // Verify first entry details
    let first_entry = &entries[0];
    assert_eq!(first_entry.id, 0);
    assert_eq!(first_entry.format, LogFormat::RFC5424);
    assert_eq!(first_entry.priority, Some(134));
    assert_eq!(first_entry.hostname.as_deref(), Some("firewall"));
    assert_eq!(first_entry.process_name.as_deref(), Some("filterlog"));
    assert_eq!(first_entry.process_id, Some(12345));
}

#[test]
fn test_parse_csv_file() {
    let fixture_path = "tests/fixtures/opnsense_filterlog.csv";
    let result = parse_file_streaming(fixture_path, LogFormat::CSV);

    assert!(result.is_ok(), "Parsing should succeed");
    let (entries, stats) = result.unwrap();

    assert_eq!(entries.len(), 5, "Should parse all 5 entries");
    assert_eq!(stats.parsed_successfully, 5);
    assert_eq!(stats.skipped_malformed, 0);
    assert_eq!(stats.format, LogFormat::CSV);

    // Verify first entry details
    let first_entry = &entries[0];
    assert_eq!(first_entry.id, 0);
    assert_eq!(first_entry.format, LogFormat::CSV);
    assert_eq!(first_entry.interface.as_deref(), Some("vtnet0"));
    assert_eq!(first_entry.action.as_deref(), Some("pass"));
    assert_eq!(first_entry.source_ip.as_deref(), Some("192.168.1.100"));
    assert_eq!(first_entry.source_port, Some(443));
    assert_eq!(first_entry.dest_ip.as_deref(), Some("10.0.0.5"));
    assert_eq!(first_entry.dest_port, Some(54321));
    assert_eq!(first_entry.protocol.as_deref(), Some("TCP"));
    assert_eq!(first_entry.rule_label.as_deref(), Some("rule1"));
}

#[test]
fn test_parser_handles_malformed_lines() {
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("mixed.log");
    let mut file = File::create(&file_path).unwrap();

    // Write mixed valid and invalid RFC3164 lines
    writeln!(file, "<134>Jan 17 12:30:45 firewall filterlog[12345]: valid entry").unwrap();
    writeln!(file, "this is not a valid log entry").unwrap();
    writeln!(file, "<134>Jan 17 12:30:46 firewall filterlog[12345]: another valid entry").unwrap();
    writeln!(file, "").unwrap(); // Empty line
    writeln!(file, "<134>Jan 17 12:30:47 firewall filterlog[12345]: third valid entry").unwrap();

    let result = parse_file_streaming(&file_path, LogFormat::RFC3164);

    assert!(result.is_ok(), "Parser should handle malformed lines gracefully");
    let (entries, stats) = result.unwrap();

    assert_eq!(entries.len(), 3, "Should parse 3 valid entries");
    assert_eq!(stats.parsed_successfully, 3);
    assert_eq!(stats.skipped_malformed, 1); // One invalid line (empty line not counted)
    assert_eq!(stats.total_lines, 5);
}

#[test]
fn test_parser_never_panics_on_empty_file() {
    use tempfile::TempDir;
    use std::fs::File;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.log");
    File::create(&file_path).unwrap();

    let result = parse_file_streaming(&file_path, LogFormat::RFC3164);

    // Parser should handle empty file without panic
    assert!(result.is_ok());
    let (entries, stats) = result.unwrap();

    assert_eq!(entries.len(), 0);
    assert_eq!(stats.total_lines, 0);
}

#[test]
fn test_format_detection_accuracy() {
    // Test RFC3164 detection
    let rfc3164_result = detect_format("tests/fixtures/opnsense_rfc3164.log");
    assert!(rfc3164_result.is_ok());
    assert_eq!(rfc3164_result.unwrap(), LogFormat::RFC3164);

    // Test RFC5424 detection
    let rfc5424_result = detect_format("tests/fixtures/opnsense_rfc5424.log");
    assert!(rfc5424_result.is_ok());
    assert_eq!(rfc5424_result.unwrap(), LogFormat::RFC5424);

    // Test CSV detection
    let csv_result = detect_format("tests/fixtures/opnsense_filterlog.csv");
    assert!(csv_result.is_ok());
    assert_eq!(csv_result.unwrap(), LogFormat::CSV);
}
