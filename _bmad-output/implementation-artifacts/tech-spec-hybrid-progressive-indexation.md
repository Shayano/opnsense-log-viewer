---
title: 'Hybrid Log Import with Progressive Indexation and Persistent Cache'
slug: 'hybrid-progressive-indexation'
created: '2026-01-21'
status: 'ready-for-dev'
stepsCompleted: [1, 2, 3, 4]
tech_stack: ['rust', 'tauri-v2', 'rkyv', 'memmap2', 'rayon', 'tokio', 'react-18', 'typescript-5.7', 'zustand']
files_to_modify:
  - 'src-tauri/src/indexer/streaming.rs'
  - 'src-tauri/src/indexer/tiered.rs'
  - 'src-tauri/src/indexer/hybrid.rs'
  - 'src-tauri/src/indexer/mod.rs'
  - 'src-tauri/src/indexer/progress.rs'
  - 'src-tauri/src/indexer/inverted.rs'
  - 'src-tauri/src/indexer/bitmap.rs'
  - 'src-tauri/src/indexer/offset_table.rs'
  - 'src-tauri/src/commands/indexation.rs'
  - 'src-tauri/Cargo.toml'
  - 'src/stores/file-store.ts'
  - 'src/components/indexation-progress/indexation-progress.tsx'
code_patterns:
  - 'Progress callback: FnMut(IndexProgress)'
  - 'Tauri events: app.emit("event-name", &payload)'
  - 'Lazy-load caching: if cached.is_none() { load(); }'
  - 'Atomic cancellation: AtomicBool + Ordering::Relaxed'
  - 'Error handling: thiserror for library, Result<T, String> for IPC'
  - 'serde rename_all: camelCase for IPC structs'
test_patterns:
  - 'Inline #[cfg(test)] mod tests in each module'
  - 'Property-based testing with proptest for parser'
  - 'Criterion benchmarks with performance gates'
---

# Tech-Spec: Hybrid Log Import with Progressive Indexation and Persistent Cache

**Created:** 2026-01-21
**Status:** Ready for Development

## Overview

### Problem Statement

Loading large OPNsense log files (14GB+, 70M+ entries) currently causes:
1. **Excessive RAM usage**: 14GB file → 20GB RAM (1.64x multiplier)
2. **Long wait times**: ~10 minutes indexation with no usability during process
3. **No feedback**: User sees only "opening" with no progress indication
4. **No caching**: Reloading the same file requires full re-indexation

Story 6.1 Tasks 1-3 implemented streaming and tiered architecture, but the final merge phase still accumulates all indexes in RAM (streaming.rs:646-662), and there's no persistent cache for subsequent loads.

### Solution

Implement a **progressive indexation architecture** with three key improvements:

1. **Progressive Indexation**: Process file in ~1GB batches, enabling partial filtering after each batch completes (~1 minute for first batch). User can start working while indexation continues in background.

2. **Persistent Index Cache (rkyv)**: Save completed indexes to disk using rkyv zero-copy serialization. Subsequent loads of the same file take ~100ms instead of 10 minutes.

3. **Real-time Progress UI**: Display detailed indexation progress with percentage, entry counts, and "partial filtering available" notifications.

### Scope

**In Scope:**
- Progressive batch indexation with early usability (filtering available per-batch)
- rkyv-based persistent index cache with file hash verification
- TieredIndex integration during streaming merge (replaces Story 6.1 Task 6)
- Real-time progress feedback in UI (percentage, entry count, status)
- File hash (SHA256) for cache invalidation
- Warm tier persistence using rkyv instead of bincode

**Out of Scope:**
- Complete parser refactoring (current parsers are adequate)
- Log format modifications
- Manual memory budget enforcement (Story 6.1 Task 4 - handled by design)
- Two-pass line-index-only approach (not suited for filtering priority)

## Context for Development

### Codebase Patterns

**From project-context.md (MANDATORY):**

| Rule | Implementation |
|------|----------------|
| **Error handling** | `thiserror` for library code, `Result<T, String>` for Tauri IPC |
| **Serde JSON** | ALWAYS `#[serde(rename_all = "camelCase")]` for IPC structs |
| **Memory safety** | Streaming for files >1GB, chunked reads with BufReader |
| **Async** | Tokio with specific features only (NOT "full") |
| **Event naming** | kebab-case for Tauri events: `indexation-progress` |
| **Command naming** | snake_case for Rust functions: `build_hybrid_index` |
| **Performance gates** | Indexation <7 sec/GB, Query <750ms, Memory <600 MB |

**Existing Progress Pattern (indexation.rs:321-324):**
```rust
let _ = app_clone.emit("indexation-progress", &progress);
```

**Lazy-Load Caching Pattern (tiered.rs:432-496):**
```rust
fn load_inverted(&mut self) -> Result<&InvertedIndex, IndexError> {
    if self.cached_inverted.is_none() {
        self.cached_inverted = Some(loaded);
    }
    Ok(self.cached_inverted.as_ref().unwrap())
}
```

### Files to Reference

| File | Purpose | Key Lines |
|------|---------|-----------|
| `src-tauri/src/indexer/streaming.rs` | Batch processing & disk serialization | 505-518 (save), 521-531 (load), 646-662 (merge bottleneck) |
| `src-tauri/src/indexer/tiered.rs` | Hot/Warm tier architecture | 288-356 (create), 432-496 (lazy-load), 731-882 (query executor) |
| `src-tauri/src/indexer/hybrid.rs` | Strategy selection & orchestration | 27 (STREAMING_THRESHOLD), 100-143 (build_index) |
| `src-tauri/src/commands/indexation.rs` | Tauri IPC commands | 245-404 (build_hybrid_index), 321-324 (progress emit) |
| `src-tauri/src/indexer/progress.rs` | Progress struct definition | 5-11 (IndexProgress struct) |
| `src/stores/file-store.ts` | Frontend state management | 7-28 (FileState), 33-44 (actions) |
| `src/components/indexation-progress/indexation-progress.tsx` | Progress UI | 35-49 (event listeners), 67-102 (display) |

### Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Serialization format | **rkyv 0.7** | Zero-copy deserialization: ~100ms load vs ~2s with bincode for 70M entries |
| Cache invalidation | **SHA256 file hash** | Reliable detection of file changes, already have `sha2` crate |
| Progress granularity | **Per-batch (~1GB)** | Balance between feedback frequency and overhead |
| Partial filtering | **Enabled per-batch** | User can filter indexed portions while rest loads |
| Hot tier default | **5M entries** | Keeps ~40MB in RAM for instant queries on recent entries |
| Warm tier format | **rkyv + mmap** | Zero-copy access, lazy loading, minimal RAM footprint |

## Implementation Plan

### Tasks

#### Phase 1: rkyv Foundation (Dependencies & Types)

- [ ] **Task 1: Add rkyv dependencies**
  - File: `src-tauri/Cargo.toml`
  - Action: Add `rkyv = { version = "0.7.45", features = ["validation", "std", "hashmap_impl"] }` and `bytecheck = "0.7.0"`
  - Notes: Keep bincode temporarily for backward compatibility during migration

- [ ] **Task 2: Derive rkyv traits on InvertedIndex**
  - File: `src-tauri/src/indexer/inverted.rs`
  - Action: Add `#[derive(Archive, rkyv::Serialize, rkyv::Deserialize)]` to `InvertedIndex` struct
  - Notes: Use `#[archive_attr(derive(CheckBytes))]` for validation support

- [ ] **Task 3: Derive rkyv traits on BitmapIndex**
  - File: `src-tauri/src/indexer/bitmap.rs`
  - Action: Add rkyv derives to `BitmapIndex`. RoaringBitmap needs custom serialization via `#[with(RoaringBitmapWrapper)]`
  - Notes: Create `RoaringBitmapWrapper` that serializes to/from bytes

- [ ] **Task 4: Derive rkyv traits on OffsetTable**
  - File: `src-tauri/src/indexer/offset_table.rs`
  - Action: Add rkyv derives to `OffsetTable` (simple Vec<u64>, straightforward)
  - Notes: This is the simplest migration

#### Phase 2: Streaming Migration (bincode → rkyv)

- [ ] **Task 5: Create rkyv serialization helpers**
  - File: `src-tauri/src/indexer/persistence.rs` (new file)
  - Action: Create `save_index_rkyv()` and `load_index_rkyv()` helper functions with validation
  - Notes: Include file magic bytes "OPNSRKYV" and version header for future compatibility

- [ ] **Task 6: Migrate batch serialization in streaming.rs**
  - File: `src-tauri/src/indexer/streaming.rs`
  - Action: Replace `save_batch_to_disk()` (lines 505-518) to use rkyv instead of bincode
  - Notes: Use `rkyv::to_bytes::<_, 256>()` with aligned buffer

- [ ] **Task 7: Migrate batch deserialization in streaming.rs**
  - File: `src-tauri/src/indexer/streaming.rs`
  - Action: Replace `load_batch_from_disk()` (lines 521-531) to use rkyv with validation
  - Notes: Use `rkyv::check_archived_root::<BatchIndexes>()` for safe loading

#### Phase 3: Progressive Indexation with Partial Filtering

- [ ] **Task 8: Extend IndexProgress for partial availability**
  - File: `src-tauri/src/indexer/progress.rs`
  - Action: Add fields: `entries_indexed: u64`, `total_entries_estimated: u64`, `partial_filter_available: bool`, `batches_completed: u32`, `total_batches: u32`
  - Notes: Keep backward compatible with existing fields

- [ ] **Task 9: Implement progressive merge with TieredIndex output**
  - File: `src-tauri/src/indexer/streaming.rs`
  - Action: Refactor final merge (lines 646-662) to:
    1. After each batch merge, emit progress with `partial_filter_available: true`
    2. Write older entries to WarmIndex file incrementally (not accumulate in RAM)
    3. Keep only last 5M entries in HotIndex
  - Notes: This is the core memory optimization - replaces Story 6.1 Task 6

- [ ] **Task 10: Create ProgressiveIndex wrapper**
  - File: `src-tauri/src/indexer/progressive.rs` (new file)
  - Action: Create `ProgressiveIndex` that wraps partial TieredIndex and allows queries on indexed portion while indexation continues
  - Notes: Must be thread-safe (Arc<RwLock<TieredIndex>>)

#### Phase 4: Persistent Cache System

- [ ] **Task 11: Implement file hash calculation**
  - File: `src-tauri/src/indexer/cache.rs` (new file)
  - Action: Create `calculate_file_hash()` using SHA256 (first 1MB + last 1MB + file size for speed)
  - Notes: Full file hash too slow for 14GB+, partial hash sufficient for change detection

- [ ] **Task 12: Implement index cache manager**
  - File: `src-tauri/src/indexer/cache.rs`
  - Action: Create `IndexCache` with methods:
    - `get_cached_index(file_path) -> Option<TieredIndex>` - checks hash, returns if valid
    - `save_index(file_path, index)` - saves with hash metadata
    - `invalidate(file_path)` - removes cached index
  - Notes: Cache location: `{app_data}/index_cache/{file_hash}.rkyv`

- [ ] **Task 13: Integrate cache into hybrid.rs orchestration**
  - File: `src-tauri/src/indexer/hybrid.rs`
  - Action: Modify `build_index()` (lines 100-143) to:
    1. First check cache for valid index
    2. If cache hit: load via rkyv (fast path ~100ms)
    3. If cache miss: run progressive indexation, save to cache on completion
  - Notes: Emit `"index-cache-hit"` or `"index-cache-miss"` event for UI feedback

#### Phase 5: Warm Tier rkyv Migration

- [ ] **Task 14: Migrate WarmIndex serialization to rkyv**
  - File: `src-tauri/src/indexer/tiered.rs`
  - Action: Replace bincode in `WarmIndex::create()` (lines 288-356) with rkyv serialization
  - Notes: Keep same file format structure (header + sections), just change encoding

- [ ] **Task 15: Migrate WarmIndex deserialization to rkyv**
  - File: `src-tauri/src/indexer/tiered.rs`
  - Action: Replace bincode in lazy-load methods (lines 432-496) with rkyv zero-copy access
  - Notes: With rkyv, can access archived data directly from mmap without full deserialization

#### Phase 6: Frontend Progressive UI

- [ ] **Task 16: Extend FileState for progressive loading**
  - File: `src/stores/file-store.ts`
  - Action: Add to interface: `indexProgress: IndexProgress | null`, `partialFilterAvailable: boolean`, `cacheHit: boolean`
  - Notes: Update actions to handle new state

- [ ] **Task 17: Update progress component for partial availability**
  - File: `src/components/indexation-progress/indexation-progress.tsx`
  - Action:
    1. Show "Partial filtering available" badge when `partial_filter_available: true`
    2. Display batches progress: "Batch 3/14 complete"
    3. Show cache hit indicator: "Loaded from cache (instant)"
  - Notes: Use existing Tailwind classes, follow component patterns

- [ ] **Task 18: Add cache hit event listener**
  - File: `src/components/indexation-progress/indexation-progress.tsx` or `src/App.tsx`
  - Action: Listen to `"index-cache-hit"` event and display toast: "Index loaded from cache"
  - Notes: Use react-hot-toast as per project patterns

#### Phase 7: Testing & Validation

- [ ] **Task 19: Add rkyv round-trip unit tests**
  - File: `src-tauri/src/indexer/persistence.rs`
  - Action: Add tests for serialization/deserialization of all index types
  - Notes: Test with realistic data sizes (100K+ entries)

- [ ] **Task 20: Add progressive indexation integration test**
  - File: `src-tauri/tests/progressive_indexation.rs` (new file)
  - Action: Test that partial filtering works during indexation, memory stays bounded
  - Notes: Use test fixture with ~100MB log file

- [ ] **Task 21: Add cache invalidation tests**
  - File: `src-tauri/src/indexer/cache.rs`
  - Action: Test cache hit, cache miss, cache invalidation on file change
  - Notes: Mock file system for fast tests

- [ ] **Task 22: Add performance benchmark for rkyv vs bincode**
  - File: `src-tauri/benches/serialization_benchmarks.rs`
  - Action: Benchmark load time for 1M, 10M, 70M entry indexes
  - Notes: Validate <100ms target for cache load

### Acceptance Criteria

#### AC1: Progressive Filtering Available
- [ ] **Given** a 14GB log file is being indexed, **when** the first batch (~1GB) completes, **then** the UI shows "Partial filtering available" and the user can filter the indexed portion while indexation continues.

#### AC2: Memory Bounded During Indexation
- [ ] **Given** a 14GB log file is being indexed, **when** the progressive indexation runs, **then** peak RAM usage stays below 2GB throughout the entire process.

#### AC3: Cache Hit Performance
- [ ] **Given** a log file was previously indexed, **when** the same file is opened again (unchanged), **then** the index loads from cache in <500ms and the UI shows "Loaded from cache".

#### AC4: Cache Invalidation on File Change
- [ ] **Given** a cached index exists for a file, **when** the file content changes (different hash), **then** the cache is invalidated and full re-indexation occurs.

#### AC5: Progress Feedback
- [ ] **Given** indexation is in progress, **when** each batch completes, **then** the UI updates with: percentage, entries indexed, batches completed (e.g., "45% - 32M/71M entries - Batch 6/14").

#### AC6: Query Performance Maintained
- [ ] **Given** a fully indexed 14GB file using TieredIndex, **when** a query is executed, **then** response time is <750ms (meeting existing performance gate).

#### AC7: Warm Tier Zero-Copy Access
- [ ] **Given** a TieredIndex with warm tier on disk, **when** querying older entries, **then** rkyv provides zero-copy access without full deserialization into RAM.

## Additional Context

### Dependencies

**Add to Cargo.toml:**
```toml
rkyv = { version = "0.7.45", features = ["validation", "std", "hashmap_impl"] }
bytecheck = "0.7.0"
```

**Existing dependencies to leverage:**
- `memmap2 = "0.9.5"` - Already used for warm tier mmap
- `rayon = "1.10.0"` - Parallel processing
- `sha2 = "0.10.8"` - Already present for file hashing
- `lasso = "0.7.3"` - String interning (Story 6.1)

**Migration note:**
- Keep `bincode` temporarily during migration for backward compatibility
- Remove bincode dependency once all migrations complete and tests pass

### Testing Strategy

**Unit Tests:**
- rkyv serialization/deserialization round-trip for InvertedIndex, BitmapIndex, OffsetTable
- RoaringBitmap custom serialization wrapper
- File hash calculation (partial hash strategy)
- Cache hit/miss/invalidation logic

**Integration Tests:**
- Progressive indexation with partial filtering verification
- Full indexation flow with TieredIndex output
- Cache persistence across app restarts

**Performance Benchmarks (criterion):**
- rkyv vs bincode: serialize 10M entries
- rkyv vs bincode: deserialize 10M entries
- Index load from cache: target <100ms for 70M entries
- Query latency with tiered index: target <750ms

**Manual Testing:**
- Open 14GB file, verify partial filtering works after ~1 minute
- Close and reopen same file, verify instant load from cache
- Modify file content, verify cache invalidation

### Notes

**Relationship to Story 6.1:**
- Builds on Tasks 1-3 (streaming, interner, tiered architecture) - COMPLETED
- Replaces Task 4 (memory budget) - handled by progressive design
- Replaces Task 6 (TieredIndex merge) - new rkyv-based approach
- Task 5 (benchmarks) - extended with rkyv benchmarks

**Key Metrics Targets:**

| Metric | Current | Target |
|--------|---------|--------|
| First filter available | ~10 min | ~1 min |
| Full indexation (14GB) | ~10 min | ~3-5 min |
| Reload same file | ~10 min | <500ms |
| Peak RAM | 20GB | <2GB |

**Risk Assessment:**

| Risk | Mitigation |
|------|------------|
| rkyv learning curve | Start with simple types (OffsetTable), progress to complex |
| RoaringBitmap rkyv compat | Custom wrapper with byte serialization fallback |
| Cache corruption | Validation on load, auto-invalidate on error |
| Partial index queries | Clear UI indication of indexed vs non-indexed entries |

**Future Considerations (Out of Scope):**
- Index compression (zstd on warm tier files)
- Incremental indexation (append-only for growing logs)
- Multi-file index merging
- Cloud sync of index cache
