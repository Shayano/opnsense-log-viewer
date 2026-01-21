use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use opnsense_log_viewer_lib::indexer::{HybridIndex, IndexCache, TieredIndex, TieredConfig, HotIndex};
use opnsense_log_viewer_lib::indexer::inverted::InvertedIndex;
use opnsense_log_viewer_lib::indexer::bitmap::BitmapIndex;
use opnsense_log_viewer_lib::indexer::offset_table::OffsetTable;
use opnsense_log_viewer_lib::types::log_entry::LogFormat;
use rkyv::Deserialize as RkyvDeserialize; // For deserialize method
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

/// Story 6.6 (AC4, AC5): Benchmark rkyv serialization performance
///
/// These benchmarks verify:
/// - AC4: rkyv is at least 10x faster than bincode for deserialization
/// - AC5: rkyv load time scales linearly and meets <100ms target for large indexes
fn benchmark_rkyv_serialization(c: &mut Criterion) {
    use opnsense_log_viewer_lib::indexer::tiered::HotIndexRkyv;

    let mut group = c.benchmark_group("rkyv");

    // Helper to create test tiered index
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

    // Benchmark rkyv serialization (10K entries)
    group.bench_function(BenchmarkId::new("serialize", "10k_entries"), |b| {
        let index = create_test_tiered_index(10_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&hot_rkyv)).unwrap()
        });
    });

    // Benchmark rkyv deserialization (10K entries)
    group.bench_function(BenchmarkId::new("deserialize", "10k_entries"), |b| {
        let index = create_test_tiered_index(10_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);
        let bytes = rkyv::to_bytes::<_, 256>(&hot_rkyv).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<HotIndexRkyv>(black_box(&bytes)).unwrap();
            archived.to_hot_index()
        });
    });

    // Benchmark rkyv serialization (50K entries) - larger dataset
    group.sample_size(10);
    group.bench_function(BenchmarkId::new("serialize", "50k_entries"), |b| {
        let index = create_test_tiered_index(50_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&hot_rkyv)).unwrap()
        });
    });

    // Benchmark rkyv deserialization (50K entries)
    group.bench_function(BenchmarkId::new("deserialize", "50k_entries"), |b| {
        let index = create_test_tiered_index(50_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);
        let bytes = rkyv::to_bytes::<_, 256>(&hot_rkyv).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<HotIndexRkyv>(black_box(&bytes)).unwrap();
            archived.to_hot_index()
        });
    });

    // Benchmark rkyv serialization (100K entries) - target scale
    group.bench_function(BenchmarkId::new("serialize", "100k_entries"), |b| {
        let index = create_test_tiered_index(100_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&hot_rkyv)).unwrap()
        });
    });

    // Benchmark rkyv deserialization (100K entries)
    group.bench_function(BenchmarkId::new("deserialize", "100k_entries"), |b| {
        let index = create_test_tiered_index(100_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);
        let bytes = rkyv::to_bytes::<_, 256>(&hot_rkyv).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<HotIndexRkyv>(black_box(&bytes)).unwrap();
            archived.to_hot_index()
        });
    });

    // Benchmark rkyv serialization (1M entries) - scaling validation
    group.bench_function(BenchmarkId::new("serialize", "1m_entries"), |b| {
        let index = create_test_tiered_index(1_000_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&hot_rkyv)).unwrap()
        });
    });

    // Benchmark rkyv deserialization (1M entries) - scaling validation for AC5
    group.bench_function(BenchmarkId::new("deserialize", "1m_entries"), |b| {
        let index = create_test_tiered_index(1_000_000);
        let hot_rkyv = HotIndexRkyv::from_hot_index(&index.hot);
        let bytes = rkyv::to_bytes::<_, 256>(&hot_rkyv).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<HotIndexRkyv>(black_box(&bytes)).unwrap();
            archived.to_hot_index()
        });
    });

    group.finish();
}

/// Story 6.6 (AC4): Benchmark rkyv vs bincode comparison
///
/// Validates that rkyv deserialization is at least 10x faster than bincode.
fn benchmark_rkyv_vs_bincode(c: &mut Criterion) {
    use opnsense_log_viewer_lib::indexer::tiered::HotIndexRkyv;

    let mut group = c.benchmark_group("rkyv_vs_bincode");
    group.sample_size(10);

    // Helper to create test index
    fn create_test_index(entry_count: u64) -> (InvertedIndex, BitmapIndex, OffsetTable) {
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

        (inverted, bitmap, offsets)
    }

    // Test with 100K entries - large enough for meaningful comparison
    let entry_count = 100_000u64;
    let (inverted, bitmap, offsets) = create_test_index(entry_count);

    // Create HotIndex
    let hot = HotIndex::from_indexes(
        inverted.clone(),
        bitmap.clone(),
        offsets.clone(),
        0,
        entry_count,
    );

    // === BINCODE BENCHMARKS ===
    // Serialize with bincode
    let bincode_bytes = bincode::serde::encode_to_vec(&hot, bincode::config::standard())
        .expect("bincode serialization");

    group.bench_function(BenchmarkId::new("bincode_deserialize", "100k"), |b| {
        b.iter(|| {
            let (decoded, _): (HotIndex, _) = bincode::serde::decode_from_slice(
                black_box(&bincode_bytes),
                bincode::config::standard(),
            ).unwrap();
            decoded
        });
    });

    // === RKYV BENCHMARKS ===
    // Serialize with rkyv
    let hot_rkyv = HotIndexRkyv::from_hot_index(&hot);
    let rkyv_bytes = rkyv::to_bytes::<_, 256>(&hot_rkyv).expect("rkyv serialization");

    group.bench_function(BenchmarkId::new("rkyv_deserialize", "100k"), |b| {
        b.iter(|| {
            let archived = rkyv::check_archived_root::<HotIndexRkyv>(black_box(&rkyv_bytes)).unwrap();
            archived.to_hot_index()
        });
    });

    // Report sizes for context
    eprintln!(
        "\n[AC4 Comparison] bincode size: {} bytes, rkyv size: {} bytes",
        bincode_bytes.len(),
        rkyv_bytes.len()
    );

    group.finish();
}

/// Story 6.6 (AC5): Benchmark cache load at scale (1M entries)
///
/// Validates cache load time scales appropriately for large indexes.
/// Note: 70M entries would take too long in CI, so we test 1M and extrapolate.
fn benchmark_cache_load_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_scaling");
    group.sample_size(10);

    // Helper to create test tiered index
    fn create_scaled_index(entry_count: u64) -> TieredIndex {
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

    // Benchmark full cache cycle for 1M entries
    group.bench_function(BenchmarkId::new("full_cache_cycle", "1m_entries"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let cache = IndexCache::new(temp_dir.path().to_path_buf());
        let log_path = temp_dir.path().join("test.log");
        std::fs::write(&log_path, "test content for 1M benchmark").unwrap();

        let index = create_scaled_index(1_000_000);
        cache.save_index(&log_path, &index).unwrap();

        b.iter(|| {
            cache.get_cached_index(black_box(&log_path)).unwrap()
        });
    });

    group.finish();
}

/// Story 6.6: Benchmark individual index component rkyv operations
fn benchmark_rkyv_components(c: &mut Criterion) {
    use opnsense_log_viewer_lib::indexer::bitmap::BitmapIndexRkyv;

    let mut group = c.benchmark_group("rkyv_components");
    group.sample_size(10);

    // Benchmark InvertedIndex rkyv (100K entries)
    // InvertedIndex derives rkyv directly
    group.bench_function(BenchmarkId::new("inverted_serialize", "100k"), |b| {
        let mut inverted = InvertedIndex::new();
        for i in 0..100_000u64 {
            let ip = format!("192.168.{}.{}", (i / 256) % 256, i % 256);
            inverted.add_entry(i, Some(&ip), Some("10.0.0.1"), Some(443), Some(80));
        }

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&inverted)).unwrap()
        });
    });

    group.bench_function(BenchmarkId::new("inverted_deserialize", "100k"), |b| {
        let mut inverted = InvertedIndex::new();
        for i in 0..100_000u64 {
            let ip = format!("192.168.{}.{}", (i / 256) % 256, i % 256);
            inverted.add_entry(i, Some(&ip), Some("10.0.0.1"), Some(443), Some(80));
        }
        let bytes = rkyv::to_bytes::<_, 256>(&inverted).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<InvertedIndex>(black_box(&bytes)).unwrap();
            let deserialized: InvertedIndex = archived.deserialize(&mut rkyv::Infallible).unwrap();
            deserialized
        });
    });

    // Benchmark BitmapIndex rkyv (100K entries)
    // BitmapIndex uses a separate BitmapIndexRkyv struct due to RoaringBitmap
    group.bench_function(BenchmarkId::new("bitmap_serialize", "100k"), |b| {
        let mut bitmap = BitmapIndex::new();
        for i in 0..100_000u64 {
            bitmap.add_entry(
                i,
                Some(if i % 3 == 0 { "block" } else { "pass" }),
                Some("TCP"),
                Some("vtnet0"),
            );
        }
        let rkyv_format = bitmap.to_rkyv();

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&rkyv_format)).unwrap()
        });
    });

    group.bench_function(BenchmarkId::new("bitmap_deserialize", "100k"), |b| {
        let mut bitmap = BitmapIndex::new();
        for i in 0..100_000u64 {
            bitmap.add_entry(
                i,
                Some(if i % 3 == 0 { "block" } else { "pass" }),
                Some("TCP"),
                Some("vtnet0"),
            );
        }
        let rkyv_format = bitmap.to_rkyv();
        let bytes = rkyv::to_bytes::<_, 256>(&rkyv_format).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<BitmapIndexRkyv>(black_box(&bytes)).unwrap();
            BitmapIndex::from_archived_rkyv(archived)
        });
    });

    // Benchmark OffsetTable rkyv (100K entries)
    // OffsetTable derives rkyv directly
    group.bench_function(BenchmarkId::new("offsets_serialize", "100k"), |b| {
        let mut offsets = OffsetTable::new();
        for i in 0..100_000u64 {
            offsets.add_offset(i, i * 100);
        }

        b.iter(|| {
            rkyv::to_bytes::<_, 256>(black_box(&offsets)).unwrap()
        });
    });

    group.bench_function(BenchmarkId::new("offsets_deserialize", "100k"), |b| {
        let mut offsets = OffsetTable::new();
        for i in 0..100_000u64 {
            offsets.add_offset(i, i * 100);
        }
        let bytes = rkyv::to_bytes::<_, 256>(&offsets).unwrap();

        b.iter(|| {
            let archived = rkyv::check_archived_root::<OffsetTable>(black_box(&bytes)).unwrap();
            let deserialized: OffsetTable = archived.deserialize(&mut rkyv::Infallible).unwrap();
            deserialized
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
    benchmark_cache_operations,
    benchmark_rkyv_serialization,
    benchmark_rkyv_vs_bincode,
    benchmark_cache_load_scaling,
    benchmark_rkyv_components
);
criterion_main!(benches);
