use peak_alloc::PeakAlloc;
use opnsense_log_viewer_lib::indexer::HybridIndex;
use opnsense_log_viewer_lib::types::log_entry::LogFormat;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

fn generate_test_log_file(entry_count: usize) -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.log");
    let mut file = File::create(&file_path).unwrap();

    // Generate RFC3164 entries
    for i in 0..entry_count {
        let day = (i % 28) + 1;
        let hour = i % 24;
        let minute = i % 60;
        let second = i % 60;
        let source_ip = format!("192.168.{}.{}", (i / 256) % 256, i % 256);
        let dest_ip = format!("10.0.{}.{}", (i / 256) % 256, i % 256);
        let source_port = 1024 + (i % 64000);
        let dest_port = 80 + (i % 20);
        let action = if i % 3 == 0 { "block" } else { "pass" };
        let protocol = if i % 2 == 0 { "TCP" } else { "UDP" };
        let interface = if i % 5 == 0 { "vtnet0" } else { "vtnet1" };

        writeln!(
            file,
            "<134>Jan {} {:02}:{:02}:{:02} firewall filterlog[123]: action={} protocol={} interface={} src={}:{} dst={}:{}",
            day, hour, minute, second, action, protocol, interface, source_ip, source_port, dest_ip, dest_port
        ).unwrap();
    }

    (temp_dir, file_path.to_string_lossy().to_string())
}

#[test]
fn test_indexation_memory_usage_small() {
    PEAK_ALLOC.reset_peak_usage();

    let (_temp_dir, file_path) = generate_test_log_file(10000); // ~1MB
    let mut index = HybridIndex::new();
    index.build_index(
        file_path,
        LogFormat::RFC3164,
        |_| {},
    ).unwrap();

    let peak_memory_bytes = PEAK_ALLOC.peak_usage();
    let peak_memory_mb = peak_memory_bytes / (1024 * 1024);
    println!("Peak memory usage (10K entries): {} MB", peak_memory_mb);

    // Assert peak memory is below 100 MB for 10K entries (reasonable for small files)
    assert!(peak_memory_mb < 100, "Memory usage exceeded 100 MB: {} MB", peak_memory_mb);
}

#[test]
fn test_indexation_memory_usage_medium() {
    PEAK_ALLOC.reset_peak_usage();

    let (_temp_dir, file_path) = generate_test_log_file(100000); // ~10MB
    let mut index = HybridIndex::new();
    index.build_index(
        file_path,
        LogFormat::RFC3164,
        |_| {},
    ).unwrap();

    let peak_memory_bytes = PEAK_ALLOC.peak_usage();
    let peak_memory_mb = peak_memory_bytes / (1024 * 1024);
    println!("Peak memory usage (100K entries): {} MB", peak_memory_mb);

    // Assert peak memory is below 250 MB for 100K entries
    assert!(peak_memory_mb < 250, "Memory usage exceeded 250 MB: {} MB", peak_memory_mb);
}

#[test]
fn test_indexation_memory_usage_large() {
    PEAK_ALLOC.reset_peak_usage();

    let (_temp_dir, file_path) = generate_test_log_file(500000); // ~50MB
    let mut index = HybridIndex::new();
    index.build_index(
        file_path,
        LogFormat::RFC3164,
        |_| {},
    ).unwrap();

    let peak_memory_bytes = PEAK_ALLOC.peak_usage();
    let peak_memory_mb = peak_memory_bytes / (1024 * 1024);
    println!("Peak memory usage (500K entries): {} MB", peak_memory_mb);

    // Assert peak memory is below 500 MB (NFR-001.4)
    assert!(peak_memory_mb < 500, "Memory usage exceeded 500 MB: {} MB", peak_memory_mb);
}
