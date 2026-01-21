# Story 6.1: Memory-Efficient Large File Indexation

Status: in-progress

## Story

As a network administrator,
I want to index large log files (14GB+) without exceeding reasonable memory limits,
so that I can analyze massive log datasets on standard hardware without running out of RAM.

## Problem Statement

**Current Behavior:**
- 14GB log file causes 23GB RAM usage (1.64x multiplier)
- 71M entries being processed
- Phase 3 (building indexes) causes memory explosion
- CPU utilization drops to 6% during Phase 3/4 indicating memory bandwidth bottleneck
- 30GB file would require ~50GB RAM - exceeding typical workstation capacity

**Expected Behavior (per NFR-001.4):**
- Peak memory during indexing: ≤500 MB
- Peak memory during search: ≤300 MB
- Stream mode if exceeding 500 MB

**Root Cause Analysis (from Party Mode discussion):**
1. All `ParsedEntry` vectors (1433 chunks × entries) kept in memory until Phase 3 completes
2. String allocations for IPs/interfaces are individual heap allocations (fragmentation)
3. `HashMap<String, Vec<u64>>` overhead massive for 70M+ entries
4. Phase 4 previously cloned HashMaps (fixed in commit c66189b) but still loads everything in RAM
5. **Memory allocation contention** (Murat): All threads allocating simultaneously causes lock contention on global allocator
6. **False sharing** (Murat): HashMaps sharing cache lines causing CPU cache invalidation
7. **Windows memory behavior** (Murat): Windows releases memory slowly (~20MB/s observed during memory descent)

## Acceptance Criteria

### AC1: Streaming Chunk Processing
**Given** a log file larger than 1GB is being indexed
**When** Phase 1-3 processing occurs
**Then** memory usage never exceeds 2GB peak
**And** chunks are processed and indexes written to disk incrementally
**And** each chunk's memory is freed before processing the next batch

### AC2: String Interning for High-Frequency Values
**Given** entries contain repeated IP addresses and interface names
**When** the indexer processes entries
**Then** string interning is used to deduplicate IP/interface strings
**And** memory usage for string storage is reduced by at least 40%

### AC3: Tiered Index Architecture
**Given** a user loads a 14GB+ log file
**When** the indexation completes
**Then** the most recent entries (configurable, default: last 5M) are kept in RAM (Hot Tier)
**And** older entries are stored in memory-mapped index files on disk (Warm Tier)
**And** queries transparently merge results from both tiers

### AC4: Memory Budget Enforcement
**Given** the application is indexing a file
**When** memory usage approaches 500MB threshold
**Then** the indexer automatically flushes partial indexes to disk
**And** processing continues in streaming mode
**And** no OOM errors occur for files up to 50GB

### AC5: Performance Targets Met
**Given** a 14GB log file with 71M entries
**When** indexation completes
**Then** total time is ≤168 seconds (≤12 sec/GB, allowing for streaming overhead)
**And** query response time remains <750ms
**And** peak RAM usage is ≤2GB (vs current 23GB)

## Tasks / Subtasks

- [ ] Task 1: Implement streaming chunk-by-chunk processing (AC: #1)
  - [ ] 1.1: Modify `parallel.rs` to process chunks in batches of 100 instead of all at once
  - [ ] 1.2: Add intermediate disk persistence after each batch
  - [ ] 1.3: Implement partial index merge for batched processing
  - [ ] 1.4: Free ParsedEntry memory immediately after index building per batch
  - [ ] 1.5: Use per-thread allocator (`mimalloc`) to avoid global allocator contention (Murat)

- [ ] Task 2: Implement string interning for IPs and interfaces (AC: #2)
  - [ ] 2.1: Benchmark `lasso` vs `Box<str>` vs custom interner (Amelia) - choose best option
  - [ ] 2.2: Create `StringInterner` struct using selected approach
  - [ ] 2.3: Replace `String` with `StringKey` (u32) in `ParsedEntry`
  - [ ] 2.4: Update `InvertedIndex` to use interned strings
  - [ ] 2.5: Pre-allocate HashMaps with estimated capacity based on file size (Amelia)
  - [ ] 2.6: Benchmark memory reduction (target: 40%+)

- [ ] Task 3: Implement Tiered Index Architecture (AC: #3)
  - [ ] 3.1: Define `HotIndex` (in-memory) and `WarmIndex` (memory-mapped) structs
  - [ ] 3.2: Implement configurable hot tier size (default: 5M entries or 500MB)
  - [ ] 3.3: Create memory-mapped index file format using `memmap2`
  - [ ] 3.4: Implement `TieredQueryExecutor` that merges hot + warm results
  - [ ] 3.5: Add automatic tier promotion/demotion based on access patterns (optional)

- [ ] Task 4: Memory budget enforcement (AC: #4)
  - [ ] 4.1: Add memory monitoring using `sysinfo` or `memory-stats` crate
  - [ ] 4.2: Implement backpressure mechanism when approaching memory limit
  - [ ] 4.3: Add automatic flush-to-disk when memory exceeds 80% of budget
  - [ ] 4.4: Test with 30GB+ synthetic log file

- [ ] Task 5: Performance validation and benchmarking (AC: #5)
  - [ ] 5.1: Create benchmark suite for large file indexation
  - [ ] 5.2: Add granular metrics: HashMap::insert vs String::clone vs RoaringBitmap::insert times (Murat)
  - [ ] 5.3: Add memory profiling to CI/CD gates
  - [ ] 5.4: Validate query performance remains <750ms with tiered architecture
  - [ ] 5.5: Document performance characteristics in README

## Dev Notes

### Relevant Architecture Patterns and Constraints

**From project-context.md:**
- ✅ MUST use streaming for files >1GB
- ✅ Use chunked reads with `BufReader`
- ✅ LRU cache with strict memory limits (500 MB max)
- ❌ NEVER load full dataset into memory

**Current Implementation (src-tauri/src/indexer/parallel.rs):**
- 4-phase architecture: Parse → Calculate IDs → Build Indexes → Merge
- Problem: Phase 1 keeps ALL ParsedEntry in memory until Phase 3 completes
- Phase 4 fixed (commit c66189b) but doesn't solve memory issue

### Proposed Solution Architecture

```
┌─────────────────────────────────────────────────────┐
│              STREAMING INDEXATION                    │
├─────────────────────────────────────────────────────┤
│  Batch 1 (100 chunks)                               │
│    Parse → Build Index → Flush to Disk → Free RAM   │
├─────────────────────────────────────────────────────┤
│  Batch 2 (100 chunks)                               │
│    Parse → Build Index → Merge with Disk → Free RAM │
├─────────────────────────────────────────────────────┤
│  ...                                                │
├─────────────────────────────────────────────────────┤
│  Final Merge → Tiered Index                         │
│    ├── Hot Tier (RAM): Last 5M entries              │
│    └── Warm Tier (mmap): Older entries              │
└─────────────────────────────────────────────────────┘
```

### Key Dependencies to Add

```toml
# Cargo.toml additions
lasso = "0.7"           # String interning (fast, memory-efficient) - evaluate vs Box<str>
memmap2 = "0.9"         # Memory-mapped files (already in use)
sysinfo = "0.31"        # Memory monitoring
mimalloc = "0.1"        # Per-thread allocator to avoid contention (Murat)
```

### Party Mode Agent Contributions Summary

| Agent | Key Contribution | Task Reference |
|-------|------------------|----------------|
| **Winston** | Tiered Architecture (Hot/Warm), Streaming, Backpressure | Task 1, 3, 4 |
| **Amelia** | String interning, `Box<str>` alternative, HashMap pre-allocation | Task 2.1, 2.5 |
| **Murat** | Allocator contention, False sharing, Granular metrics | Task 1.5, 5.2 |

### Project Structure Notes

Files to modify:
- `src-tauri/src/indexer/parallel.rs` - Main streaming logic
- `src-tauri/src/indexer/hybrid.rs` - Tiered index orchestration
- `src-tauri/src/indexer/bitmap.rs` - Add disk persistence methods
- `src-tauri/src/indexer/inverted.rs` - Add disk persistence methods
- `src-tauri/src/indexer/mod.rs` - Export new tiered types

New files to create:
- `src-tauri/src/indexer/interner.rs` - String interning
- `src-tauri/src/indexer/tiered.rs` - Hot/Warm tier management
- `src-tauri/src/indexer/disk_index.rs` - Memory-mapped index format

### References

- [Source: project-context.md#Memory Safety (CRITICAL)]
- [Source: architecture.md#NFR-001.4]
- [Source: epics.md#NFR-001.4: Memory Efficiency]
- [Source: Party Mode Discussion - Winston, Amelia, Murat analysis]

### Performance Targets

| Metric | Current | Target | Method |
|--------|---------|--------|--------|
| Peak RAM (14GB file) | 23 GB | ≤2 GB | Streaming + Tiered |
| Indexation Speed | ~138s Phase 1 | ≤168s total | Batched processing |
| Query Latency | <500ms | <750ms | Tiered query merge |
| 30GB File Support | OOM | Works | Memory budget enforcement |

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Party Mode discussion: 2026-01-21
- Phase 4 fix commit: c66189b (move semantics)
- Performance logs showing 23GB RAM for 14GB file

### Completion Notes List

_(To be filled during implementation)_

### File List

_(To be filled during implementation)_
