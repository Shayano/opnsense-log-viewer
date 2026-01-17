use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Placeholder function for indexing - will be implemented in Story 1.3
fn placeholder_index_data(data: &str) -> usize {
    // Simulate indexing by counting lines
    data.lines().count()
}

// Placeholder function for simple query - will be implemented in Story 2.2
fn placeholder_simple_query(data: &str, _filter: &str) -> Vec<String> {
    // Simulate simple query by returning first 10 lines
    data.lines().take(10).map(|s| s.to_string()).collect()
}

// Placeholder function for complex query - will be implemented in Story 2.2
fn placeholder_complex_query(data: &str, _filters: &[&str]) -> Vec<String> {
    // Simulate complex query with multiple filters
    data.lines()
        .filter(|line| line.len() > 100) // Placeholder filtering logic
        .take(10)
        .map(|s| s.to_string())
        .collect()
}

// Generate synthetic log data for benchmarking
fn generate_synthetic_logs(size_mb: usize) -> String {
    let line_count = (size_mb * 1024 * 1024) / 200; // Assume ~200 bytes per line
    let mut data = String::with_capacity(size_mb * 1024 * 1024);

    for i in 0..line_count {
        // Realistic OPNsense filterlog CSV format
        data.push_str(&format!(
            "5,,,{},vtnet0,match,block,in,4,0x0,,64,{},0,none,6,tcp,60,192.168.1.{},203.0.113.{},{},{},0,S,{},,64240,,mss;sackOK;TS\n",
            1000000000 + i,
            i % 65535,
            (i % 254) + 1,
            (i % 254) + 1,
            (i % 60000) + 1024,
            (i % 1000) + 80,
            i * 12345
        ));
    }

    data
}

fn benchmark_indexation(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexation");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark 1GB indexation (target: <7 sec/GB = <7 sec for 1GB)
    let data_1gb = generate_synthetic_logs(1024); // 1GB

    group.bench_with_input(BenchmarkId::new("index", "1GB"), &data_1gb, |b, data| {
        b.iter(|| {
            placeholder_index_data(black_box(data))
        });
    });

    group.finish();
}

fn benchmark_simple_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("queries");
    group.measurement_time(Duration::from_secs(5));

    let data_1gb = generate_synthetic_logs(1024);

    // Simple query benchmark (target: <500ms)
    group.bench_function("simple_query", |b| {
        b.iter(|| {
            placeholder_simple_query(black_box(&data_1gb), black_box("block"))
        });
    });

    group.finish();
}

fn benchmark_complex_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_queries");
    group.measurement_time(Duration::from_secs(5));

    let data_1gb = generate_synthetic_logs(1024);

    // Complex query with 5+ filters (target: <750ms)
    group.bench_function("complex_query_5_filters", |b| {
        let filters = vec!["block", "vtnet0", "tcp", "192.168", "443"];
        b.iter(|| {
            placeholder_complex_query(black_box(&data_1gb), black_box(&filters))
        });
    });

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");
    group.measurement_time(Duration::from_secs(5));

    // Memory usage benchmark (target: <600 MB peak)
    // Note: criterion doesn't measure memory directly, but we can simulate operations
    group.bench_function("memory_during_indexation", |b| {
        b.iter(|| {
            let data = generate_synthetic_logs(100); // 100MB chunks
            placeholder_index_data(black_box(&data))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_indexation,
    benchmark_simple_query,
    benchmark_complex_query,
    benchmark_memory_usage
);
criterion_main!(benches);
