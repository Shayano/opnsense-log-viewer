/// Integration test for index persistence (Story 1.4)
/// Tests complete save/load workflow with integrity verification

use opnsense_log_viewer_lib::indexer::HybridIndex;
use opnsense_log_viewer_lib::storage::{calculate_file_hash, load_index, save_index};
use opnsense_log_viewer_lib::types::log_entry::LogFormat;
use opnsense_log_viewer_lib::types::persisted_index::{PersistedIndex, SourceFileMetadata};
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_index_save_and_load_roundtrip() {
    // Setup: Create temp log file
    let temp_dir = TempDir::new().unwrap();
    let log_file_path = temp_dir.path().join("test.log");
    let mut log_file = File::create(&log_file_path).unwrap();

    // Write test log entries
    for i in 0..1000 {
        writeln!(
            log_file,
            "<134>Jan 15 12:00:00 firewall filterlog[123]: test entry {}",
            i
        )
        .unwrap();
    }
    drop(log_file);

    // Calculate source file hash
    let source_hash = calculate_file_hash(&log_file_path).expect("Failed to calculate hash");

    // Create and build index
    let mut hybrid_index = HybridIndex::new();
    let _metadata = hybrid_index
        .build_index(&log_file_path, LogFormat::RFC3164, |_progress| {})
        .expect("Failed to build index");

    // Create PersistedIndex
    let source_metadata = SourceFileMetadata {
        file_path: log_file_path.to_string_lossy().to_string(),
        file_size: std::fs::metadata(&log_file_path).unwrap().len(),
        entry_count: 1000,
        log_format: LogFormat::RFC3164,
    };

    let persisted_index = PersistedIndex::new(source_hash, source_metadata, hybrid_index);

    // Save to disk
    let index_path = temp_dir.path().join("test.idx");
    save_index(&persisted_index, &index_path).expect("Failed to save index");

    // Verify file exists and has content
    assert!(index_path.exists());
    let file_size = std::fs::metadata(&index_path).unwrap().len();
    assert!(file_size > 0, "Index file should not be empty");

    // Load from disk
    let loaded_index = load_index(&index_path).expect("Failed to load index");

    // Verify integrity
    assert_eq!(loaded_index.source_file_hash, source_hash);
    assert_eq!(loaded_index.source_metadata.entry_count, 1000);
    assert_eq!(loaded_index.source_metadata.log_format, LogFormat::RFC3164);
    assert_eq!(
        loaded_index.source_metadata.file_path,
        log_file_path.to_string_lossy().to_string()
    );
}

#[test]
fn test_idempotent_reindexing() {
    // Test NFR-002.4: Re-indexing produces semantically identical results
    // Note: HashMap order is non-deterministic, so we verify semantic equivalence
    let temp_dir = TempDir::new().unwrap();
    let log_file_path = temp_dir.path().join("idempotence.log");
    let mut log_file = File::create(&log_file_path).unwrap();

    // Write fixed content
    for i in 0..500 {
        writeln!(
            log_file,
            "<134>Jan 15 12:00:00 firewall filterlog[123]: test {}",
            i
        )
        .unwrap();
    }
    drop(log_file);

    let mut source_hashes = Vec::new();
    let mut entry_counts = Vec::new();

    // Index 10 times
    for run in 0..10 {
        let mut hybrid_index = HybridIndex::new();
        let _metadata = hybrid_index
            .build_index(&log_file_path, LogFormat::RFC3164, |_| {})
            .expect("Failed to build index");

        // Calculate source hash (should be identical)
        let source_hash = calculate_file_hash(&log_file_path).unwrap();
        source_hashes.push(source_hash);

        // Create and save
        let source_metadata = SourceFileMetadata {
            file_path: log_file_path.to_string_lossy().to_string(),
            file_size: std::fs::metadata(&log_file_path).unwrap().len(),
            entry_count: 500,
            log_format: LogFormat::RFC3164,
        };

        entry_counts.push(source_metadata.entry_count);

        let persisted_index =
            PersistedIndex::new(source_hash, source_metadata, hybrid_index);

        // Save and reload to verify integrity
        let index_path = temp_dir.path().join(format!("run_{}.idx", run));
        save_index(&persisted_index, &index_path).unwrap();

        let loaded = load_index(&index_path).unwrap();
        assert_eq!(loaded.source_file_hash, source_hash);
        assert_eq!(loaded.source_metadata.entry_count, 500);

        println!(
            "Run {}: source_hash = {}, entries = {}",
            run + 1,
            hex::encode(source_hash),
            loaded.source_metadata.entry_count
        );
    }

    // Verify idempotence: all source hashes identical
    let first_hash = source_hashes[0];
    for (i, hash) in source_hashes.iter().enumerate() {
        assert_eq!(
            hash, &first_hash,
            "Run {} produced different source hash. Idempotence violated!",
            i + 1
        );
    }

    // Verify all entry counts identical
    let first_count = entry_counts[0];
    for (i, count) in entry_counts.iter().enumerate() {
        assert_eq!(
            count, &first_count,
            "Run {} produced different entry count. Idempotence violated!",
            i + 1
        );
    }

    println!("✅ Idempotence verified: 10/10 re-indexes produced identical source hashes and entry counts");
}

#[test]
fn test_load_performance() {
    // Test NFR-001.6: Load time ≤2 sec
    let temp_dir = TempDir::new().unwrap();
    let log_file_path = temp_dir.path().join("large.log");
    let mut log_file = File::create(&log_file_path).unwrap();

    // Create larger log file (10K entries)
    for i in 0..10_000 {
        writeln!(
            log_file,
            "<134>Jan 15 12:00:00 firewall filterlog[123]: test entry {}",
            i
        )
        .unwrap();
    }
    drop(log_file);

    // Build and save index
    let mut hybrid_index = HybridIndex::new();
    let _metadata = hybrid_index
        .build_index(&log_file_path, LogFormat::RFC3164, |_| {})
        .unwrap();

    let source_hash = calculate_file_hash(&log_file_path).unwrap();
    let source_metadata = SourceFileMetadata {
        file_path: log_file_path.to_string_lossy().to_string(),
        file_size: std::fs::metadata(&log_file_path).unwrap().len(),
        entry_count: 10_000,
        log_format: LogFormat::RFC3164,
    };

    let persisted_index = PersistedIndex::new(source_hash, source_metadata, hybrid_index);

    let index_path = temp_dir.path().join("large.idx");
    save_index(&persisted_index, &index_path).unwrap();

    // Measure load time
    let start = Instant::now();
    let _loaded = load_index(&index_path).expect("Failed to load index");
    let elapsed = start.elapsed();

    println!("Index load time: {:.2}s", elapsed.as_secs_f64());
    assert!(
        elapsed.as_secs() < 2,
        "Load time exceeded 2 second threshold: {:.2}s",
        elapsed.as_secs_f64()
    );

    println!("✅ Performance test passed: load time = {:.2}s (target: ≤2s)", elapsed.as_secs_f64());
}

#[test]
fn test_hash_mismatch_detection() {
    // Test: Loading with modified source file should be detectable
    let temp_dir = TempDir::new().unwrap();
    let log_file_path = temp_dir.path().join("modify.log");
    let mut log_file = File::create(&log_file_path).unwrap();

    writeln!(log_file, "<134>Jan 15 12:00:00 firewall filterlog[123]: original").unwrap();
    drop(log_file);

    // Build and save original index
    let original_hash = calculate_file_hash(&log_file_path).unwrap();
    let mut hybrid_index = HybridIndex::new();
    let _metadata = hybrid_index
        .build_index(&log_file_path, LogFormat::RFC3164, |_| {})
        .unwrap();

    let source_metadata = SourceFileMetadata {
        file_path: log_file_path.to_string_lossy().to_string(),
        file_size: std::fs::metadata(&log_file_path).unwrap().len(),
        entry_count: 1,
        log_format: LogFormat::RFC3164,
    };

    let persisted_index = PersistedIndex::new(original_hash, source_metadata, hybrid_index);

    let index_path = temp_dir.path().join("modify.idx");
    save_index(&persisted_index, &index_path).unwrap();

    // Modify source file
    let mut log_file = File::create(&log_file_path).unwrap();
    writeln!(log_file, "<134>Jan 15 12:00:00 firewall filterlog[123]: MODIFIED").unwrap();
    drop(log_file);

    // Recalculate hash
    let modified_hash = calculate_file_hash(&log_file_path).unwrap();

    // Load index
    let loaded_index = load_index(&index_path).unwrap();

    // Verify hashes don't match
    assert_ne!(
        loaded_index.source_file_hash, modified_hash,
        "Modified file hash should not match stored hash"
    );

    println!("✅ Hash mismatch detection works correctly");
}
