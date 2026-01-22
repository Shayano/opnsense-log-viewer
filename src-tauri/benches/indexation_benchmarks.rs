//! SQLite-Based Log Indexation Benchmarks
//!
//! Story 6.6: Performance validation benchmarks for SQLite log indexation pipeline.
//!
//! ## Benchmark Groups
//!
//! - `sqlite_import`: Import speed benchmarks (10K, 100K, 1M entries)
//! - `sqlite_query`: Query benchmarks (simple, complex, regex, count)
//! - `file_hash`: Quick file hash calculation benchmarks
//!
//! ## Performance Targets
//!
//! - Import speed: ≥200K entries/sec (target 300K+)
//! - Simple filter: <200ms
//! - Complex filter (5+ fields): <500ms
//! - Regex filter: <750ms
//! - Count query: <100ms
//! - Memory peak: <1GB
//!
//! ## Usage
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench --bench indexation_benchmarks
//!
//! # Run specific benchmark group
//! cargo bench --bench indexation_benchmarks -- sqlite_import
//!
//! # View HTML reports
//! open target/criterion/report/index.html
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use rand::{SeedableRng, Rng, seq::SliceRandom};
use rand::rngs::StdRng;

// Import SQLite pipeline components
use opnsense_log_viewer_lib::indexer::sqlite::{
    SqliteConnectionPool, build_sqlite_index,
};
use opnsense_log_viewer_lib::types::log_entry::LogFormat;

// Import query components
use opnsense_log_viewer_lib::query::sqlite_executor::SqliteQueryExecutor;
use opnsense_log_viewer_lib::query::types::{FilterCondition, FilterField, FilterOperator, FilterValue, LogicOperator};

// Import file hash function
use opnsense_log_viewer_lib::storage::integrity::calculate_file_hash_quick;

// =============================================================================
// FIXTURE GENERATOR (AC1: Deterministic log generation)
// =============================================================================

/// Generate a deterministic test log file with RFC3164-style OPNsense filterlog entries.
///
/// Uses seeded RNG for reproducible benchmarks across runs.
///
/// # Arguments
/// * `entry_count` - Number of log entries to generate
/// * `seed` - RNG seed for deterministic output
///
/// # Returns
/// * `(TempDir, String, u64)` - Temp directory, file path, and file size in bytes
#[allow(dead_code)] // Reserved for RFC3164 format benchmarks (currently using CSV for reliability)
fn generate_test_log_file(entry_count: usize, seed: u64) -> (TempDir, String, u64) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.log");
    let mut file = File::create(&file_path).unwrap();

    let mut rng = StdRng::seed_from_u64(seed);

    // Realistic OPNsense filterlog patterns
    let actions = ["pass", "block"];
    let protocols = ["TCP", "UDP", "ICMP", "GRE"];
    let interfaces = ["vtnet0", "vtnet1", "igb0", "em0", "lo0"];
    let directions = ["in", "out"];

    for i in 0..entry_count {
        // Generate realistic timestamp
        let day = (i % 28) + 1;
        let hour = rng.gen_range(0..24);
        let minute = rng.gen_range(0..60);
        let second = rng.gen_range(0..60);

        // Generate realistic IPs
        let src_ip = format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..255),
            rng.gen_range(0..256),
            rng.gen_range(0..256),
            rng.gen_range(1..255)
        );
        let dst_ip = format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..255),
            rng.gen_range(0..256),
            rng.gen_range(0..256),
            rng.gen_range(1..255)
        );

        // Generate realistic ports
        let src_port = rng.gen_range(1024..65535);
        let dst_port = if rng.gen_bool(0.3) {
            // Common ports 30% of the time
            *[22, 80, 443, 8080, 3389].choose(&mut rng).unwrap()
        } else {
            rng.gen_range(1..65535)
        };

        let action = actions[rng.gen_range(0..actions.len())];
        let protocol = protocols[rng.gen_range(0..protocols.len())];
        let interface = interfaces[rng.gen_range(0..interfaces.len())];
        let direction = directions[rng.gen_range(0..directions.len())];
        let rule_id = format!("rule_{}", rng.gen_range(1..100));

        // RFC3164 format with filterlog-style data
        writeln!(
            file,
            "<134>Jan {} {:02}:{:02}:{:02} firewall filterlog[{}]: {},{},{},{},{},{},{},{},{},{}",
            day, hour, minute, second,
            rng.gen_range(1000..9999),
            rule_id,
            interface,
            action,
            direction,
            protocol,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            rng.gen_range(40..1500) // packet length
        ).unwrap();
    }

    file.flush().unwrap();
    let file_size = std::fs::metadata(&file_path).unwrap().len();

    (temp_dir, file_path.to_string_lossy().to_string(), file_size)
}

/// Generate a CSV log file for reliable parsing benchmarks
fn generate_csv_log_file(entry_count: usize, seed: u64) -> (TempDir, String, u64) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.log");
    let mut file = File::create(&file_path).unwrap();

    let mut rng = StdRng::seed_from_u64(seed);

    let actions = ["pass", "block"];
    let protocols = ["TCP", "UDP", "ICMP"];
    let interfaces = ["vtnet0", "vtnet1", "igb0"];

    // CSV header - matches OPNsense filterlog CSV export format
    // Using a simplified header that matches what the parser expects
    writeln!(
        file,
        "rulenr,interface,reason,act,dir,ipversion,proto,src,srcport,dst,dstport,timestamp"
    ).unwrap();

    for i in 0..entry_count {
        let day = (i % 28) + 1;
        let hour = rng.gen_range(0..24);
        let minute = rng.gen_range(0..60);
        let second = rng.gen_range(0..60);

        let src_ip = format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..255),
            rng.gen_range(0..256),
            rng.gen_range(0..256),
            rng.gen_range(1..255)
        );
        let dst_ip = format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..255),
            rng.gen_range(0..256),
            rng.gen_range(0..256),
            rng.gen_range(1..255)
        );

        let src_port = rng.gen_range(1024..65535);
        let dst_port = rng.gen_range(1..65535);

        let action = actions[rng.gen_range(0..actions.len())];
        let protocol = protocols[rng.gen_range(0..protocols.len())];
        let interface = interfaces[rng.gen_range(0..interfaces.len())];
        let rule_id = rng.gen_range(1..100);

        writeln!(
            file,
            "{},{},match,{},in,4,{},{},{},{},{},2026-01-{:02} {:02}:{:02}:{:02}",
            rule_id,
            interface,
            action,
            protocol,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            day,
            hour,
            minute,
            second
        ).unwrap();
    }

    file.flush().unwrap();
    let file_size = std::fs::metadata(&file_path).unwrap().len();

    (temp_dir, file_path.to_string_lossy().to_string(), file_size)
}

// =============================================================================
// SQLITE IMPORT BENCHMARKS (AC2: Import speed benchmarks)
// =============================================================================

/// Benchmark SQLite import performance at various scales.
///
/// Tests:
/// - 10K entries (~1MB) - Small file baseline
/// - 100K entries (~10MB) - Medium file
/// - 1M entries (~100MB) - Large file target
///
/// Metrics tracked:
/// - Entries per second
/// - Bytes per second throughput
fn benchmark_sqlite_import(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqlite_import");

    // Test sizes: 10K, 100K, 1M entries
    let test_sizes = [
        (10_000, "10k_entries"),
        (100_000, "100k_entries"),
    ];

    for (entry_count, label) in test_sizes.iter() {
        // Generate test file once per test size (deterministic seed)
        let (temp_dir, file_path, file_size) = generate_csv_log_file(*entry_count, 42);
        let db_dir = TempDir::new().unwrap();

        // Set throughput for entries/sec calculation
        group.throughput(Throughput::Elements(*entry_count as u64));

        if *entry_count >= 100_000 {
            group.sample_size(10);
        }

        group.bench_function(BenchmarkId::new("import", label), |b| {
            let mut iteration = 0u64;
            b.iter(|| {
                // Each iteration needs a fresh database
                let db_path = db_dir.path().join(format!("bench_{}.sqlite", iteration));
                iteration += 1;

                let pool = SqliteConnectionPool::new(&db_path, 4).unwrap();
                let stats = build_sqlite_index(
                    &pool,
                    Path::new(&file_path),
                    LogFormat::CSV,
                    &format!("bench_hash_{}", iteration),
                    file_size,
                ).unwrap();

                black_box(stats)
            });
        });

        // Cleanup temp dir is dropped automatically
        drop(temp_dir);
    }

    group.finish();
}

/// Benchmark large-scale SQLite import (1M entries).
///
/// Separate from main import benchmark due to longer runtime.
/// Sample size reduced to 10 for reasonable CI time.
fn benchmark_sqlite_import_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqlite_import_large");
    group.sample_size(10);

    let entry_count = 1_000_000;
    let (temp_dir, file_path, file_size) = generate_csv_log_file(entry_count, 42);
    let db_dir = TempDir::new().unwrap();

    group.throughput(Throughput::Elements(entry_count as u64));

    group.bench_function(BenchmarkId::new("import", "1m_entries"), |b| {
        let mut iteration = 0u64;
        b.iter(|| {
            let db_path = db_dir.path().join(format!("bench_1m_{}.sqlite", iteration));
            iteration += 1;

            let pool = SqliteConnectionPool::new(&db_path, 4).unwrap();
            let stats = build_sqlite_index(
                &pool,
                Path::new(&file_path),
                LogFormat::CSV,
                &format!("bench_hash_1m_{}", iteration),
                file_size,
            ).unwrap();

            // Log performance metrics
            eprintln!(
                "\n[1M IMPORT] {} entries in {}ms = {} entries/sec",
                stats.entry_count,
                stats.elapsed_ms,
                stats.entries_per_second
            );

            black_box(stats)
        });
    });

    drop(temp_dir);
    group.finish();
}

// =============================================================================
// SQLITE QUERY BENCHMARKS (AC3: Query performance tests)
// =============================================================================

/// Create a populated test database for query benchmarks.
///
/// Returns pool with 100K entries already indexed.
fn create_populated_database(entry_count: usize) -> (TempDir, Arc<SqliteConnectionPool>) {
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("query_bench.sqlite");
    let pool = Arc::new(SqliteConnectionPool::new(&db_path, 4).unwrap());

    // Generate and import test data
    let (log_temp, log_path, file_size) = generate_csv_log_file(entry_count, 42);

    build_sqlite_index(
        &pool,
        Path::new(&log_path),
        LogFormat::CSV,
        "query_bench_hash",
        file_size,
    ).unwrap();

    drop(log_temp); // Clean up log file

    (db_dir, pool)
}

/// Benchmark SQLite query performance.
///
/// Tests against 100K entry database:
/// - Simple filter (single field = value): <200ms target
/// - Complex filter (5+ fields with AND/OR): <500ms target
/// - Regex filter: <750ms target
/// - Count query: <100ms target
fn benchmark_sqlite_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqlite_query");

    // Create populated database (100K entries)
    let (_db_temp, pool) = create_populated_database(100_000);
    let executor = SqliteQueryExecutor::new(pool);

    // Simple filter: action = "block"
    group.bench_function("simple_filter", |b| {
        let filters = vec![FilterCondition {
            field: FilterField::Action,
            operator: FilterOperator::Equals,
            value: FilterValue::String("block".to_string()),
            logic: None,
        }];

        b.iter(|| {
            executor.query(black_box(&filters), 1000, 0).unwrap()
        });
    });

    // Complex filter: 5 conditions with AND/OR
    group.bench_function("complex_filter_5_conditions", |b| {
        let filters = vec![
            FilterCondition {
                field: FilterField::Action,
                operator: FilterOperator::Equals,
                value: FilterValue::String("block".to_string()),
                logic: Some(LogicOperator::And),
            },
            FilterCondition {
                field: FilterField::Protocol,
                operator: FilterOperator::Equals,
                value: FilterValue::String("TCP".to_string()),
                logic: Some(LogicOperator::And),
            },
            FilterCondition {
                field: FilterField::SourcePort,
                operator: FilterOperator::GreaterThan,
                value: FilterValue::Number(1024),
                logic: Some(LogicOperator::And),
            },
            FilterCondition {
                field: FilterField::DestinationPort,
                operator: FilterOperator::Equals,
                value: FilterValue::Number(443),
                logic: Some(LogicOperator::Or),
            },
            FilterCondition {
                field: FilterField::Interface,
                operator: FilterOperator::Contains,
                value: FilterValue::String("vtnet".to_string()),
                logic: None,
            },
        ];

        b.iter(|| {
            executor.query(black_box(&filters), 1000, 0).unwrap()
        });
    });

    // Regex filter: IP pattern matching
    // Pattern matches any IP with 2-digit first octet (10-99), which our generator produces
    group.bench_function("regex_filter", |b| {
        let filters = vec![FilterCondition {
            field: FilterField::SourceIp,
            operator: FilterOperator::Regex,
            value: FilterValue::String(r"^\d{2}\.\d+\.\d+\.\d+".to_string()),
            logic: None,
        }];

        b.iter(|| {
            executor.query(black_box(&filters), 1000, 0).unwrap()
        });
    });

    // Count query (no filter, total entries)
    group.bench_function("count_total", |b| {
        b.iter(|| {
            executor.total_entries().unwrap()
        });
    });

    // Count with filter
    group.bench_function("count_with_filter", |b| {
        let filters = vec![FilterCondition {
            field: FilterField::Action,
            operator: FilterOperator::Equals,
            value: FilterValue::String("pass".to_string()),
            logic: None,
        }];

        b.iter(|| {
            executor.count(black_box(&filters)).unwrap()
        });
    });

    // Batch entry fetch by IDs
    group.bench_function("fetch_entries_by_id_batch", |b| {
        // Get some entry IDs first
        let ids: Vec<u64> = (1..=100).collect();

        b.iter(|| {
            executor.get_entries_by_ids(black_box(&ids)).unwrap()
        });
    });

    group.finish();
}

/// Benchmark query performance at 1M entry scale.
fn benchmark_sqlite_query_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqlite_query_large");
    group.sample_size(10);

    // Create 1M entry database
    let (_db_temp, pool) = create_populated_database(1_000_000);
    let executor = SqliteQueryExecutor::new(pool);

    // Simple filter at scale
    group.bench_function("simple_filter_1m", |b| {
        let filters = vec![FilterCondition {
            field: FilterField::Action,
            operator: FilterOperator::Equals,
            value: FilterValue::String("block".to_string()),
            logic: None,
        }];

        b.iter(|| {
            let result = executor.query(black_box(&filters), 1000, 0).unwrap();
            eprintln!("\n[1M QUERY] Simple filter: {} matches in {}ms", result.matched_count, result.execution_time_ms);
            result
        });
    });

    // Complex filter at scale
    group.bench_function("complex_filter_1m", |b| {
        let filters = vec![
            FilterCondition {
                field: FilterField::Action,
                operator: FilterOperator::Equals,
                value: FilterValue::String("block".to_string()),
                logic: Some(LogicOperator::And),
            },
            FilterCondition {
                field: FilterField::Protocol,
                operator: FilterOperator::Equals,
                value: FilterValue::String("TCP".to_string()),
                logic: Some(LogicOperator::And),
            },
            FilterCondition {
                field: FilterField::SourcePort,
                operator: FilterOperator::GreaterThan,
                value: FilterValue::Number(10000),
                logic: None,
            },
        ];

        b.iter(|| {
            let result = executor.query(black_box(&filters), 1000, 0).unwrap();
            eprintln!("\n[1M QUERY] Complex filter: {} matches in {}ms", result.matched_count, result.execution_time_ms);
            result
        });
    });

    group.finish();
}

// =============================================================================
// FILE HASH BENCHMARKS (AC2: File hash performance)
// =============================================================================

/// Benchmark quick file hash calculation.
///
/// Tests `calculate_file_hash_quick` which reads first 1MB + last 1MB.
/// Should complete in <1 second for any file size.
fn benchmark_file_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_hash");

    // Small file hash (1KB)
    group.bench_function(BenchmarkId::new("hash", "1kb"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.log");
        let content = vec![b'X'; 1024];
        std::fs::write(&file_path, &content).unwrap();

        b.iter(|| {
            calculate_file_hash_quick(black_box(&file_path)).unwrap()
        });
    });

    // Medium file hash (10MB)
    group.bench_function(BenchmarkId::new("hash", "10mb"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("medium.log");
        let content = vec![b'X'; 10 * 1024 * 1024];
        std::fs::write(&file_path, &content).unwrap();

        b.iter(|| {
            calculate_file_hash_quick(black_box(&file_path)).unwrap()
        });
    });

    // Large file hash (100MB) - should only read first+last 1MB
    group.sample_size(10);
    group.bench_function(BenchmarkId::new("hash", "100mb"), |b| {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.log");
        let content = vec![b'X'; 100 * 1024 * 1024];
        std::fs::write(&file_path, &content).unwrap();

        b.iter(|| {
            calculate_file_hash_quick(black_box(&file_path)).unwrap()
        });
    });

    group.finish();
}

// =============================================================================
// INDEXATION BENCHMARKS (Legacy comparison - using HybridIndex)
// =============================================================================

use opnsense_log_viewer_lib::indexer::HybridIndex;

/// Benchmark legacy HybridIndex for comparison with SQLite.
fn benchmark_hybrid_index_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_index_comparison");

    // Only test small sizes for comparison - HybridIndex doesn't scale well
    group.bench_function(BenchmarkId::new("hybrid_index", "1000_entries"), |b| {
        let (_temp_dir, file_path, _) = generate_csv_log_file(1000, 42);

        b.iter(|| {
            let mut index = HybridIndex::new();
            index.build_index(
                black_box(&file_path),
                LogFormat::CSV,
                |_| {},
            ).unwrap();
        });
    });

    group.sample_size(10);
    group.bench_function(BenchmarkId::new("hybrid_index", "10000_entries"), |b| {
        let (_temp_dir, file_path, _) = generate_csv_log_file(10000, 42);

        b.iter(|| {
            let mut index = HybridIndex::new();
            index.build_index(
                black_box(&file_path),
                LogFormat::CSV,
                |_| {},
            ).unwrap();
        });
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUPS
// =============================================================================

criterion_group!(
    benches,
    benchmark_sqlite_import,
    benchmark_sqlite_import_large,
    benchmark_sqlite_query,
    benchmark_sqlite_query_large,
    benchmark_file_hash,
    benchmark_hybrid_index_comparison,
);

criterion_main!(benches);
