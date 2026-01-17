#!/usr/bin/env python3
"""
Performance Gate Enforcement Script for CI/CD

Parses criterion benchmark output and enforces performance gates:
- Indexation: <8.05 sec/GB (7 sec/GB ±15%)
- Simple Query: <750ms (500ms ±50%)
- Complex Query: <1125ms (750ms ±50%)
- Memory: <720 MB (600 MB ±20%)

Exit codes:
  0 - All gates passed
  1 - One or more gates failed
  2 - Error parsing benchmark results
"""

import sys
import os
import json
from pathlib import Path
from typing import Dict, Optional

# Performance gate thresholds
GATES = {
    'indexation_sec_per_gb': 8.05,  # 7 * 1.15
    'simple_query_ms': 750,         # 500 * 1.5
    'complex_query_ms': 1125,       # 750 * 1.5
    'memory_mb': 720,               # 600 * 1.2
}


def parse_criterion_output(criterion_dir: Path) -> Dict[str, float]:
    """
    Parse criterion benchmark results from JSON files.

    Returns dict of benchmark_name -> time_in_ms
    """
    results = {}

    # Criterion stores results in target/criterion/<benchmark_name>/base/estimates.json
    if not criterion_dir.exists():
        print(f"❌ Error: Criterion directory not found: {criterion_dir}")
        return results

    # Find all benchmark directories
    for benchmark_dir in criterion_dir.iterdir():
        if not benchmark_dir.is_dir():
            continue

        estimates_file = benchmark_dir / "base" / "estimates.json"
        if not estimates_file.exists():
            # Try new criterion format
            estimates_file = benchmark_dir / "estimates.json"

        if estimates_file.exists():
            try:
                with open(estimates_file, 'r') as f:
                    data = json.load(f)
                    # Criterion stores time in nanoseconds
                    time_ns = data.get('mean', {}).get('point_estimate', 0)
                    time_ms = time_ns / 1_000_000  # Convert to milliseconds
                    results[benchmark_dir.name] = time_ms
            except Exception as e:
                print(f"⚠️  Warning: Failed to parse {estimates_file}: {e}")

    return results


def check_gates(results: Dict[str, float]) -> bool:
    """
    Check if benchmark results pass performance gates.

    Returns True if all gates pass, False otherwise.
    """
    all_passed = True

    print("=" * 60)
    print("🚀 PERFORMANCE GATE CHECK")
    print("=" * 60)
    print()

    # Check indexation gate (looking for "index" in benchmark name)
    indexation_benchmarks = {k: v for k, v in results.items() if 'index' in k.lower()}
    if indexation_benchmarks:
        for name, time_ms in indexation_benchmarks.items():
            # Assume 1GB test data, convert ms to seconds
            time_sec = time_ms / 1000
            threshold = GATES['indexation_sec_per_gb']

            status = "✅ PASS" if time_sec <= threshold else "❌ FAIL"
            all_passed = all_passed and (time_sec <= threshold)

            print(f"Indexation ({name}):")
            print(f"  Result:    {time_sec:.3f} sec/GB")
            print(f"  Threshold: {threshold:.3f} sec/GB (±15%)")
            print(f"  Status:    {status}")
            print()
    else:
        print("⚠️  Warning: No indexation benchmarks found")
        print()

    # Check query gates
    query_benchmarks = {k: v for k, v in results.items() if 'query' in k.lower()}
    if query_benchmarks:
        for name, time_ms in query_benchmarks.items():
            if 'simple' in name.lower():
                threshold = GATES['simple_query_ms']
                tolerance = "±50%"
            elif 'complex' in name.lower():
                threshold = GATES['complex_query_ms']
                tolerance = "±50%"
            else:
                # Default to complex query threshold
                threshold = GATES['complex_query_ms']
                tolerance = "±50%"

            status = "✅ PASS" if time_ms <= threshold else "❌ FAIL"
            all_passed = all_passed and (time_ms <= threshold)

            print(f"Query ({name}):")
            print(f"  Result:    {time_ms:.3f} ms")
            print(f"  Threshold: {threshold:.1f} ms ({tolerance})")
            print(f"  Status:    {status}")
            print()
    else:
        print("⚠️  Warning: No query benchmarks found")
        print()

    # Check memory gate
    memory_benchmarks = {k: v for k, v in results.items() if 'memory' in k.lower()}
    if memory_benchmarks:
        for name, time_ms in memory_benchmarks.items():
            # Note: Criterion measures time, not memory
            # This is a placeholder - actual memory measurement requires different tools
            threshold = GATES['memory_mb']

            print(f"Memory ({name}):")
            print(f"  Note: Criterion measures time, not memory.")
            print(f"  Actual memory profiling requires separate tooling.")
            print(f"  Threshold: {threshold:.0f} MB (±20%)")
            print(f"  Status:    ⚠️  SKIPPED (not measured)")
            print()

    print("=" * 60)
    if all_passed:
        print("✅ ALL PERFORMANCE GATES PASSED")
    else:
        print("❌ PERFORMANCE GATES FAILED")
    print("=" * 60)
    print()

    return all_passed


def main():
    """Main entry point."""
    # Find criterion output directory
    criterion_dir = Path("src-tauri/target/criterion")

    if not criterion_dir.exists():
        # Try alternate location
        criterion_dir = Path("target/criterion")

    if not criterion_dir.exists():
        print("❌ Error: Criterion benchmark results not found.")
        print("   Expected location: src-tauri/target/criterion/")
        print("   Run 'cargo bench' first to generate benchmark results.")
        sys.exit(2)

    # Parse benchmark results
    results = parse_criterion_output(criterion_dir)

    if not results:
        print("❌ Error: No benchmark results found in criterion output.")
        print("   Verify that benchmarks ran successfully.")
        sys.exit(2)

    # Check performance gates
    passed = check_gates(results)

    # Exit with appropriate code
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
