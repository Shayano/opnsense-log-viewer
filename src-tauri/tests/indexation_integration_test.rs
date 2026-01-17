use opnsense_log_viewer_lib::indexer::HybridIndex;
use opnsense_log_viewer_lib::types::log_entry::LogFormat;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn generate_rfc3164_log_file(entry_count: usize) -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("rfc3164.log");
    let mut file = File::create(&file_path).unwrap();

    for i in 0..entry_count {
        let day = (i % 28) + 1;
        writeln!(
            file,
            "<134>Jan {} 00:00:00 firewall filterlog[123]: pass TCP 192.168.1.{} 10.0.0.{} {} {} vtnet0",
            day, i % 255, i % 255, 1024 + i, 80 + (i % 100)
        ).unwrap();
    }

    (temp_dir, file_path.to_string_lossy().to_string())
}

fn generate_rfc5424_log_file(entry_count: usize) -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("rfc5424.log");
    let mut file = File::create(&file_path).unwrap();

    for i in 0..entry_count {
        writeln!(
            file,
            "<134>1 2023-01-{:02}T12:00:00Z firewall filterlog 123 - - test message {}",
            (i % 28) + 1, i
        ).unwrap();
    }

    (temp_dir, file_path.to_string_lossy().to_string())
}

fn generate_csv_log_file(entry_count: usize) -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("csv.log");
    let mut file = File::create(&file_path).unwrap();

    for i in 0..entry_count {
        writeln!(
            file,
            "5,,,1000{},vtnet0,match,{},in,4,0x0,,64,{},0,none,6,tcp,60,192.168.1.{},10.0.0.{},{},{},0,S,{},,64240,,mss",
            i,
            if i % 2 == 0 { "block" } else { "pass" },
            i % 65535,
            i % 255,
            i % 255,
            1024 + i,
            80 + (i % 100),
            i * 12345
        ).unwrap();
    }

    (temp_dir, file_path.to_string_lossy().to_string())
}

#[test]
fn test_indexation_rfc3164() {
    let (_temp_dir, file_path) = generate_rfc3164_log_file(100);
    let mut index = HybridIndex::new();

    let metadata = index.build_index(
        &file_path,
        LogFormat::RFC3164,
        |_| {},
    ).unwrap();

    assert_eq!(metadata.entry_count, 100);
    assert_eq!(metadata.format, LogFormat::RFC3164);
    assert!(metadata.source_file_size > 0);
    assert!(!metadata.source_file_hash.is_empty());
}

#[test]
fn test_indexation_rfc5424() {
    let (_temp_dir, file_path) = generate_rfc5424_log_file(100);
    let mut index = HybridIndex::new();

    let metadata = index.build_index(
        &file_path,
        LogFormat::RFC5424,
        |_| {},
    ).unwrap();

    assert_eq!(metadata.entry_count, 100);
    assert_eq!(metadata.format, LogFormat::RFC5424);
    assert!(metadata.source_file_size > 0);
    assert!(!metadata.source_file_hash.is_empty());
}

#[test]
fn test_indexation_csv() {
    let (_temp_dir, file_path) = generate_csv_log_file(100);
    let mut index = HybridIndex::new();

    let metadata = index.build_index(
        &file_path,
        LogFormat::CSV,
        |_| {},
    ).unwrap();

    assert_eq!(metadata.entry_count, 100);
    assert_eq!(metadata.format, LogFormat::CSV);
    assert!(metadata.source_file_size > 0);
    assert!(!metadata.source_file_hash.is_empty());
}

#[test]
fn test_indexation_progress_events() {
    let (_temp_dir, file_path) = generate_rfc3164_log_file(1000);
    let mut index = HybridIndex::new();
    let mut progress_count = 0;

    let _metadata = index.build_index(
        &file_path,
        LogFormat::RFC3164,
        |progress| {
            progress_count += 1;
            assert!(progress.percentage >= 0.0 && progress.percentage <= 100.0);
            assert!(progress.bytes_processed <= progress.total_bytes);
        },
    ).unwrap();

    // At least one progress event should be emitted
    assert!(progress_count > 0, "No progress events emitted");
}

#[test]
fn test_indexation_cancellation() {
    let (_temp_dir, file_path) = generate_rfc3164_log_file(10000);
    let mut index = HybridIndex::new();

    // Cancel indexation immediately
    index.cancel();

    let result = index.build_index(
        &file_path,
        LogFormat::RFC3164,
        |_| {},
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cancelled"));
}

#[test]
fn test_memory_usage_tracking() {
    let (_temp_dir, file_path) = generate_rfc3164_log_file(1000);
    let mut index = HybridIndex::new();

    index.build_index(
        &file_path,
        LogFormat::RFC3164,
        |_| {},
    ).unwrap();

    let memory_usage = index.memory_usage();

    // Memory usage should be reasonable (less than 50MB for 1000 entries)
    assert!(memory_usage < 50 * 1024 * 1024, "Memory usage too high: {} bytes", memory_usage);
}
