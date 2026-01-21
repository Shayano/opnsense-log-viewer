use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use opnsense_log_viewer_lib::indexer::{HybridIndex, IndexCache, TieredIndex, TieredConfig, HotIndex};
use opnsense_log_viewer_lib::indexer::inverted::InvertedIndex;
use opnsense_log_viewer_lib::indexer::bitmap::BitmapIndex;
use opnsense_log_viewer_lib::indexer::offset_table::OffsetTable;
use opnsense_log_viewer_lib::types::log_entry::LogFormat;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

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

        writeln!(
            file,
            "<134>Jan {} {:02}:{:02}:{:02} firewall filterlog[123]: {} {} {} {} {} {}",
            day, hour, minute, second, action, protocol, source_ip, dest_ip, source_port, dest_port
        ).unwrap();
    }

    (temp_dir, file_path.to_string_lossy().to_string())
}

fn benchmark_indexation_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexation");

    // Benchmark 1000 entries (~100KB)
    group.bench_function(BenchmarkId::new("index", "1000_entries"), |b| {
        b.iter(|| {
            let (_temp_dir, file_path) = generate_test_log_file(1000);
            let mut index = HybridIndex::new();
            index.build_index(
                black_box(&file_path),
                LogFormat::RFC3164,
                |_| {},
            ).unwrap();
        });
    });

    group.finish();
}

fn benchmark_indexation_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexation");

    // Benchmark 10000 entries (~1MB)
    group.bench_function(BenchmarkId::new("index", "10000_entries"), |b| {
        b.iter(|| {
            let (_temp_dir, file_path) = generate_test_log_file(10000);
            let mut index = HybridIndex::new();
            index.build_index(
                black_box(&file_path),
                LogFormat::RFC3164,
                |_| {},
            ).unwrap();
        });
    });

    group.finish();
}

fn benchmark_indexation_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexation");
    group.sample_size(10); // Reduce sample size for large benchmark

    // Benchmark 100000 entries (~10MB)
    group.bench_function(BenchmarkId::new("index", "100000_entries"), |b| {
        b.iter(|| {
            let (_temp_dir, file_path) = generate_test_log_file(100000);
            let mut index = HybridIndex::new();
            index.build_index(
                black_box(&file_path),
                LogFormat::RFC3164,
                |_| {},
            ).unwrap();
        });
    });

    group.finish();
}

/// Story 6.4: Benchmark cache file hash calculation
fn benchmark_cache_file_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");

    // Small file hash (1KB)
    group.bench_function(BenchmarkId::new("file_hash", "1kb"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.log");
        let content = vec![b'X'; 1024]; // 1KB
        std::fs::write(&file_path, &content).unwrap();

        b.iter(|| {
            opnsense_log_viewer_lib::indexer::calculate_file_hash(black_box(&file_path)).unwrap()
        });
    });

    // Medium file hash (10MB)
    group.bench_function(BenchmarkId::new("file_hash", "10mb"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("medium.log");
        let content = vec![b'X'; 10 * 1024 * 1024]; // 10MB
        std::fs::write(&file_path, &content).unwrap();

        b.iter(|| {
            opnsense_log_viewer_lib::indexer::calculate_file_hash(black_box(&file_path)).unwrap()
        });
    });

    // Large file hash (100MB) - should only read first+last 1MB
    group.sample_size(10);
    group.bench_function(BenchmarkId::new("file_hash", "100mb"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.log");
        let content = vec![b'X'; 100 * 1024 * 1024]; // 100MB
        std::fs::write(&file_path, &content).unwrap();

        b.iter(|| {
            opnsense_log_viewer_lib::indexer::calculate_file_hash(black_box(&file_path)).unwrap()
        });
    });

    group.finish();
}

/// Story 6.4: Benchmark cache save and load operations
fn benchmark_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");

    // Create test tiered index with specified entry count
    fn create_test_tiered_index(entry_count: u64) -> TieredIndex {
        let mut inverted = InvertedIndex::new();
        let mut bitmap = BitmapIndex::new();
        let mut offsets = OffsetTable::new();

        for i in 0..entry_count {
            let ip = format!("192.168.{}.{}", (i / 256) % 256, i % 256);
            inverted.add_entry(i, Some(&ip), Some("10.0.0.1"), Some(443), Some(80));
            bitmap.add_entry(
                i,
                Some(if i % 2 == 0 { "block" } else { "pass" }),
                Some("TCP"),
                Some("vtnet0"),
            );
            offsets.add_offset(i, i * 100);
        }

        let hot = HotIndex::from_indexes(inverted, bitmap, offsets, 0, entry_count);

        TieredIndex {
            config: TieredConfig::default(),
            hot,
            warm: Vec::new(),
            total_entries: entry_count,
        }
    }

    // Benchmark cache save (10k entries)
    group.bench_function(BenchmarkId::new("save", "10k_entries"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let cache = IndexCache::new(temp_dir.path().to_path_buf());
        let log_path = temp_dir.path().join("test.log");
        std::fs::write(&log_path, "test content").unwrap();
        let index = create_test_tiered_index(10_000);

        b.iter(|| {
            cache.save_index(black_box(&log_path), black_box(&index)).unwrap();
        });
    });

    // Benchmark cache load (10k entries)
    group.bench_function(BenchmarkId::new("load", "10k_entries"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let cache = IndexCache::new(temp_dir.path().to_path_buf());
        let log_path = temp_dir.path().join("test.log");
        std::fs::write(&log_path, "test content for loading").unwrap();
        let index = create_test_tiered_index(10_000);
        cache.save_index(&log_path, &index).unwrap();

        b.iter(|| {
            cache.get_cached_index(black_box(&log_path)).unwrap()
        });
    });

    // Benchmark cache operations with larger index (100k entries)
    group.sample_size(10);
    group.bench_function(BenchmarkId::new("save", "100k_entries"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let cache = IndexCache::new(temp_dir.path().to_path_buf());
        let log_path = temp_dir.path().join("test.log");
        std::fs::write(&log_path, "test content").unwrap();
        let index = create_test_tiered_index(100_000);

        b.iter(|| {
            cache.save_index(black_box(&log_path), black_box(&index)).unwrap();
        });
    });

    group.bench_function(BenchmarkId::new("load", "100k_entries"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let cache = IndexCache::new(temp_dir.path().to_path_buf());
        let log_path = temp_dir.path().join("test.log");
        std::fs::write(&log_path, "test content for loading 100k").unwrap();
        let index = create_test_tiered_index(100_000);
        cache.save_index(&log_path, &index).unwrap();

        b.iter(|| {
            cache.get_cached_index(black_box(&log_path)).unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_indexation_small,
    benchmark_indexation_medium,
    benchmark_indexation_large,
    benchmark_cache_file_hash,
    benchmark_cache_operations
);
criterion_main!(benches);
