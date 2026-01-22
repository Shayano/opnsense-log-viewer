# Story 6.3: Real-Time Streaming Progress & Non-Blocking UI

Status: done

## Story

As a network administrator,
I want real-time progress updates during import without UI freezing,
So that I know exactly how fast indexing is progressing and can continue using the app.

## Acceptance Criteria

### AC1: Immediate UI Response on Import Start
**Given** a file import is started
**When** the import begins
**Then** immediately (within 100ms) the UI shows:
- File name and size
- "Starting import..."
- Progress bar at 0%

### AC2: Real-Time Progress Events (1 Hz)
**Given** import is in progress
**When** progress events are emitted (every 1 second)
**Then** the UI displays:
- Percentage: "45%"
- Entries processed: "32,450,000 / 71,930,527"
- Speed: "312,000 entries/sec"
- Time elapsed: "1:43"
- ETA: "~2:15 remaining"

### AC3: Async Tauri Command Pattern
**Given** the import uses Tauri async commands
**When** I implement the non-blocking pattern
**Then** `build_sqlite_index` command is marked `#[tauri::command]`
**And** progress is emitted via `app.emit("indexation-progress", payload)`
**And** UI thread is never blocked by Rust computation

### AC4: IndexProgress Struct Extended for SQLite
**Given** IndexProgress struct needs updates
**When** I extend it for SQLite streaming
**Then** it includes:
```typescript
interface IndexProgress {
    phase: "parsing" | "indexing" | "complete";
    entriesProcessed: number;
    totalEntriesEstimated: number;
    bytesProcessed: number;
    totalBytes: number;
    entriesPerSecond: number;
    elapsedSeconds: number;
    estimatedSecondsRemaining: number;
}
```

### AC5: Smooth Progress UI Updates
**Given** the frontend receives progress events
**When** it updates the progress UI
**Then** the progress bar animates smoothly
**And** numbers update every second
**And** the main window remains interactive (can minimize, move, close)

### AC6: Completion Notification
**Given** the import completes
**When** the final event is emitted
**Then** the UI shows:
- "Import complete!"
- Total entries: "71,930,527"
- Total time: "3:58"
- Average speed: "301,245 entries/sec"
**And** a toast notification appears with success message

### AC7: Progress Emission Thread (1 Hz)
**Given** the pipeline is running
**When** progress needs to be communicated to the frontend
**Then** a dedicated thread polls `PipelineProgress` atomics every 1 second
**And** emits Tauri events without blocking the parsing/writing pipeline
**And** calculates rolling average speed (5-second window) for stable display

## Tasks / Subtasks

- [x] Task 1: Create new async command `build_sqlite_index` (AC: 3)
  - [x] 1.1 Create `src-tauri/src/commands/sqlite_indexation.rs`
  - [x] 1.2 Implement `#[tauri::command] async fn build_sqlite_index(app: AppHandle, file_path: String) -> Result<SqliteIndexMetadata, String>`
  - [x] 1.3 Validate file path (reuse validation from `indexation.rs`)
  - [x] 1.4 Detect log format using existing `detect_format`
  - [x] 1.5 Call `get_or_create_database` from Story 6.1 for SQLite pool
  - [x] 1.6 Spawn pipeline orchestrator from Story 6.2 in background thread
  - [x] 1.7 Return immediately to keep UI responsive

- [x] Task 2: Implement progress emission thread (AC: 7, 2)
  - [x] 2.1 Create `src-tauri/src/commands/progress_emitter.rs` (note: placed in commands module)
  - [x] 2.2 Define `ProgressEmitter` struct with `Arc<PipelineProgress>` reference
  - [x] 2.3 Implement `run(app: AppHandle, progress: Arc<PipelineProgress>, total_bytes: u64)`
  - [x] 2.4 Poll progress atomics every 1 second (1 Hz)
  - [x] 2.5 Convert `ProgressSnapshot` to frontend-compatible `IndexProgress` struct
  - [x] 2.6 Emit via `app.emit("indexation-progress", payload)`
  - [x] 2.7 Implement rolling 5-second average for speed calculation
  - [x] 2.8 Stop emitting when pipeline completes (check `is_complete()`)

- [x] Task 3: Extend IndexProgress for SQLite metrics (AC: 4)
  - [x] 3.1 Added `entries_per_second` field (calculated from entries_indexed / elapsed_seconds)
  - [x] 3.2 Added `elapsed_seconds` field (passed through from constructor)
  - [x] 3.3 Ensure serde `rename_all = "camelCase"` is applied (already in place)
  - [x] 3.4 Update TypeScript `IndexProgress` interface in `src/types/file.ts`
  - [x] 3.5 Added `SqliteIndexMetadata` TypeScript interface

- [x] Task 4: Wire up pipeline with progress emission (AC: 1, 2, 3)
  - [x] 4.1 Command spawns progress emitter in dedicated std::thread
  - [x] 4.2 Spawn progress emitter thread before starting pipeline
  - [x] 4.3 Share `Arc<PipelineProgress>` with emitter and pipeline
  - [x] 4.4 Emit initial "Starting import..." event immediately (within 100ms)
  - [x] 4.5 Emit final "indexation-complete" event with statistics

- [x] Task 5: Update frontend progress component (AC: 5, 6)
  - [x] 5.1 Update `indexation-progress.tsx` to handle new field names (entriesIndexed, totalEntriesEstimated, batchesCompleted)
  - [x] 5.2 Add `entriesPerSecond` display with formatting (K, M suffixes)
  - [x] 5.3 Add `elapsedSeconds` display in mm:ss format
  - [x] 5.4 Add `estimatedSecondsRemaining` display (ETA)
  - [x] 5.5 Conditional rendering - fields only shown when values > 0

- [x] Task 6: Tests (AC: all)
  - [x] 6.1 IndexProgress Rust tests (10 tests pass) - entries_per_second calculation, serde serialization
  - [x] 6.2 ProgressEmitter Rust tests (7 tests pass) - rolling average, ETA calculation, emission interval
  - [x] 6.3 sqlite_indexation Rust tests (4 tests pass) - path validation, metadata conversion
  - [x] 6.4 Frontend file-store tests (16 tests pass) - IndexProgress state management
  - [x] 6.5 Frontend indexation-progress component tests (14 tests pass) - UI updates

- [x] Task 7: Register new command in Tauri (AC: 3)
  - [x] 7.1 Update `src-tauri/src/commands/mod.rs` to include `sqlite_indexation` and `progress_emitter` modules
  - [x] 7.2 Add `build_sqlite_index` and `cancel_sqlite_indexation` to `generate_handler!` macro in `lib.rs`
  - [x] 7.3 Command registered and callable from frontend

## Dev Notes

### Architecture Context

This story bridges Story 6.2's parallel parsing pipeline with the frontend UI. The key challenge is emitting progress updates without blocking the high-throughput pipeline (300K+ entries/sec).

**Architecture Diagram:**
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  File Reader    │ --> │ Bounded Channel │ --> │ SQLite Writer   │
│  (BufReader)    │     │ (crossbeam)     │     │ (Single Thread) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       ^                       │
        v                       │                       │
┌─────────────────┐             │                       │
│  Rayon Workers  │ ────────────┘                       │
│ (Parallel Parse)│                                     │
└─────────────────┘                                     │
        │                                               │
        └──────────── Arc<PipelineProgress> ───────────┘
                              │
                              v
                    ┌─────────────────┐
                    │ Progress Emitter │ (1 Hz polling)
                    │ (Separate Thread)│
                    └─────────────────┘
                              │
                              v
                    ┌─────────────────┐
                    │   Tauri Events  │ ("indexation-progress")
                    └─────────────────┘
                              │
                              v
                    ┌─────────────────┐
                    │  React Frontend │
                    └─────────────────┘
```

### Critical Technical Requirements

**DO NOT block the async runtime.** The progress emitter must run in its own `std::thread`, not on Tokio, to avoid blocking the Tauri event loop.

**Use `Arc<PipelineProgress>` for sharing.** Story 6.2 already fixed the Clone issue - progress is now shared via Arc for correct atomic updates.

**Emission frequency: 1 Hz (every 1 second).** More frequent updates waste CPU; less frequent feels laggy.

### Code Reuse - CRITICAL

**Reuse existing `PipelineProgress` from Story 6.2:**
```rust
use crate::indexer::sqlite::{
    PipelineProgress, ProgressSnapshot,
    build_sqlite_index, get_or_create_database
};
```

**Reuse existing `IndexProgress` struct (extend, don't replace):**
```rust
use crate::indexer::progress::IndexProgress;
```

**Reuse file validation from `commands/indexation.rs`:**
```rust
// Copy validation logic, don't import (avoids coupling)
// - File exists check
// - Path canonicalization
// - Read permission check
// - Format detection
```

**Reuse format detection:**
```rust
use crate::parser::detect_format;
```

### Progress Emitter Design

```rust
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use crate::indexer::sqlite::PipelineProgress;
use crate::indexer::IndexProgress;

/// Emits progress events to frontend at 1 Hz
pub struct ProgressEmitter {
    progress: Arc<PipelineProgress>,
    app: AppHandle,
    total_bytes: u64,
    total_entries_estimated: u64,
}

impl ProgressEmitter {
    pub fn new(
        app: AppHandle,
        progress: Arc<PipelineProgress>,
        total_bytes: u64,
    ) -> Self {
        // Estimate entries based on avg line length (200 bytes)
        let total_entries_estimated = total_bytes / 200;
        Self { progress, app, total_bytes, total_entries_estimated }
    }

    /// Run in dedicated thread - polls progress every 1 second
    pub fn run(self) {
        let start = Instant::now();
        let mut speed_history: Vec<u64> = Vec::with_capacity(5);

        loop {
            thread::sleep(Duration::from_secs(1));

            let snapshot = self.progress.snapshot();

            // Check if complete
            if snapshot.is_complete() {
                self.emit_completion(snapshot, start.elapsed().as_secs_f64());
                break;
            }

            // Update rolling speed average (5-second window)
            speed_history.push(snapshot.entries_per_second);
            if speed_history.len() > 5 {
                speed_history.remove(0);
            }
            let avg_speed = speed_history.iter().sum::<u64>() / speed_history.len() as u64;

            // Calculate ETA
            let remaining_entries = self.total_entries_estimated.saturating_sub(snapshot.entries_written);
            let eta_seconds = if avg_speed > 0 {
                remaining_entries as f64 / avg_speed as f64
            } else {
                0.0
            };

            // Emit progress event
            let progress_event = IndexProgress::with_batch_info(
                snapshot.bytes_read,
                self.total_bytes,
                start.elapsed().as_secs_f64(),
                snapshot.entries_written,
                self.total_entries_estimated,
                true, // partial_filter_available after first batch
                0, // batches_completed - N/A for streaming
                0, // total_batches - N/A for streaming
            );

            let _ = self.app.emit("indexation-progress", &progress_event);
        }
    }

    fn emit_completion(&self, snapshot: ProgressSnapshot, elapsed_secs: f64) {
        let final_progress = IndexProgress::with_batch_info(
            self.total_bytes,
            self.total_bytes,
            elapsed_secs,
            snapshot.entries_written,
            snapshot.entries_written, // Actual count now known
            true,
            1,
            1,
        );

        let _ = self.app.emit("indexation-progress", &final_progress);
        let _ = self.app.emit("indexation-complete", &final_progress);
    }
}
```

### Async Command Pattern

```rust
use tauri::{AppHandle, Manager, Emitter};
use std::sync::Arc;
use std::thread;

#[tauri::command]
pub async fn build_sqlite_index(
    app: AppHandle,
    file_path: String,
) -> Result<SqliteIndexMetadata, String> {
    // 1. Validate file path (sync, fast)
    let path = validate_file_path(&file_path)?;
    let file_size = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to get file size: {}", e))?
        .len();

    // 2. Detect format (sync, fast)
    let format = detect_format(&path)
        .map_err(|e| format!("Failed to detect format: {}", e))?;

    // 3. Get or create SQLite database (Story 6.1)
    let pool = get_or_create_database(&app, &path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // 4. Create shared progress tracker
    let progress = Arc::new(PipelineProgress::new());

    // 5. Emit "Starting..." immediately (AC1: within 100ms)
    let _ = app.emit("indexation-progress", &IndexProgress::new(0, file_size, 0.0));

    // 6. Spawn progress emitter in separate thread
    let emitter_progress = Arc::clone(&progress);
    let emitter_app = app.clone();
    thread::spawn(move || {
        ProgressEmitter::new(emitter_app, emitter_progress, file_size).run();
    });

    // 7. Run pipeline in blocking task (keeps async runtime free)
    let pipeline_progress = Arc::clone(&progress);
    let result = tokio::task::spawn_blocking(move || {
        crate::indexer::sqlite::build_sqlite_index(
            &pool,
            &path,
            format,
            &file_hash,
            file_size,
            pipeline_progress, // Pass Arc for progress sharing
        )
    }).await.map_err(|e| format!("Pipeline task failed: {}", e))?;

    result.map(|stats| SqliteIndexMetadata {
        entry_count: stats.entry_count,
        elapsed_ms: stats.elapsed_ms,
        entries_per_second: stats.entries_per_second,
    })
}
```

### Frontend TypeScript Interface Updates

Update `src/types/file.ts`:

```typescript
/**
 * Progress data during SQLite indexation
 * Story 6.3: Extended with entries/second and ETA
 */
export interface IndexProgress {
  /** Percentage complete (0-100) */
  percentage: number;
  /** Bytes processed so far */
  bytesProcessed: number;
  /** Total bytes to process */
  totalBytes: number;
  /** Current indexation speed in GB/min */
  speedGbps: number;
  /** Estimated seconds remaining */
  etaSeconds: number;
  /** Number of entries processed so far */
  entriesProcessed: number;
  /** Estimated total entries */
  totalEntriesEstimate: number;
  /** Current batch number (N/A for SQLite streaming) */
  currentBatch: number;
  /** Total number of batches (N/A for SQLite streaming) */
  totalBatches: number;
  /** True when filtering can begin */
  partialFilterAvailable: boolean;
  /** Story 6.3: Entries processed per second */
  entriesPerSecond: number;
  /** Story 6.3: Elapsed time in seconds */
  elapsedSeconds: number;
}
```

### Error Handling Pattern

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SqliteIndexError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Format detection failed: {0}")]
    FormatDetection(String),

    #[error("Database error: {0}")]
    Database(#[from] crate::indexer::sqlite::SqliteCacheError),

    #[error("Pipeline error: {0}")]
    Pipeline(#[from] crate::indexer::sqlite::PipelineError),
}

// Convert to String for Tauri IPC
impl From<SqliteIndexError> for String {
    fn from(e: SqliteIndexError) -> Self {
        e.to_string()
    }
}
```

### File Structure Requirements

Create new files:
```
src-tauri/src/
├── commands/
│   ├── mod.rs              # Update with sqlite_indexation module
│   └── sqlite_indexation.rs # NEW - async command handler
├── indexer/
│   ├── sqlite/
│   │   ├── mod.rs          # Update with progress_emitter
│   │   └── progress_emitter.rs # NEW - 1 Hz event emitter
```

Modify existing files:
```
src-tauri/src/
├── lib.rs                  # Add build_sqlite_index to generate_handler!
├── indexer/
│   ├── progress.rs         # Add entriesPerSecond, elapsedSeconds fields
│   ├── sqlite/
│   │   └── pipeline.rs     # Accept Arc<PipelineProgress> parameter
src/
├── types/file.ts           # Add new IndexProgress fields
├── components/indexation-progress/
│   └── indexation-progress.tsx # Display new fields
```

### Testing Requirements

**Unit Tests (90% coverage target for indexer module):**
- ProgressEmitter emits at correct frequency (~1 Hz)
- Rolling speed average calculation is correct
- ETA calculation handles edge cases (0 speed)
- IndexProgress serializes correctly with new fields

**Integration Tests:**
- Full pipeline with progress events (100MB fixture)
- Verify at least 5 progress events for 100MB file
- Verify completion event contains final statistics
- Verify UI responsiveness via Tauri mock

**Manual Testing Checklist:**
- [ ] 100MB file: Progress updates visible, UI responsive
- [ ] 1GB file: Smooth progress, accurate ETA
- [ ] Cancel during import: Progress stops, no crash
- [ ] Multiple imports: Previous progress cleared

### Performance Expectations

| Metric | Target |
|--------|--------|
| Initial event latency | <100ms |
| Progress event frequency | 1 Hz (±100ms) |
| UI frame rate during import | >30 FPS |
| Memory overhead from emitter | <1MB |
| CPU overhead from emitter | <1% |

### Project Structure Notes

- New command in `commands/sqlite_indexation.rs` follows existing command pattern
- Progress emitter in `indexer/sqlite/` alongside other pipeline components
- Uses `std::thread` for emitter (not Tokio) to avoid blocking async runtime
- Frontend component already exists - extend, don't replace

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.3] - Acceptance criteria
- [Source: _bmad-output/project-context.md#Tauri-Specific-Patterns] - IPC event patterns
- [Source: src-tauri/src/commands/indexation.rs] - Existing command pattern to follow
- [Source: src-tauri/src/indexer/sqlite/progress.rs] - PipelineProgress from Story 6.2
- [Source: src-tauri/src/indexer/progress.rs] - IndexProgress struct to extend
- [Source: src/components/indexation-progress/indexation-progress.tsx] - Frontend component
- [Source: src/types/file.ts] - TypeScript interfaces

### Integration with Previous Stories

**From Story 6.1 (SQLite Schema):**
- Use `get_or_create_database(app_handle, file_path)` for pool
- Use `SqliteConnectionPool` for database access

**From Story 6.2 (Parallel Pipeline):**
- Use `build_sqlite_index()` pipeline orchestrator
- Use `Arc<PipelineProgress>` for shared progress tracking
- Use `ProgressSnapshot` for reading progress values

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Rust compilation: All checks pass with only warnings (unused imports, etc.)
- TypeScript compilation: `npx tsc --noEmit` passes with no errors
- Rust tests: 31 relevant tests pass (IndexProgress: 10, ProgressEmitter: 7, sqlite_indexation: 4, pipeline: 10)
- Frontend tests: 30 tests pass (file-store: 16, indexation-progress: 14)

### Completion Notes List

1. **Progress Emitter placed in commands module** - Placed in `src-tauri/src/commands/progress_emitter.rs` instead of `indexer/sqlite/` because it directly depends on Tauri's `AppHandle` and `Emitter` traits, making it more suitable as a command helper.

2. **IndexProgress field name alignment** - Updated TypeScript interface to match Rust field names exactly:
   - `entriesProcessed` → `entriesIndexed`
   - `totalEntriesEstimate` → `totalEntriesEstimated`
   - `currentBatch` → `batchesCompleted`

3. **Conditional UI rendering** - Progress fields (Speed, Elapsed, ETA) are only shown when their values are greater than 0, providing a cleaner initial state.

4. **Rolling speed average** - Implemented 5-second window using `VecDeque` for stable speed display.

5. **Cancellation not yet implemented** - `cancel_sqlite_indexation` returns error; requires atomic flag in PipelineProgress (future enhancement).

### Change Log

| Date | Change | Files |
|------|--------|-------|
| 2026-01-22 | Created async command `build_sqlite_index` | `src-tauri/src/commands/sqlite_indexation.rs` |
| 2026-01-22 | Created progress emitter with 1 Hz polling | `src-tauri/src/commands/progress_emitter.rs` |
| 2026-01-22 | Extended IndexProgress with entries_per_second, elapsed_seconds | `src-tauri/src/indexer/progress.rs` |
| 2026-01-22 | Updated commands module | `src-tauri/src/commands/mod.rs` |
| 2026-01-22 | Registered new commands in Tauri | `src-tauri/src/lib.rs` |
| 2026-01-22 | Updated TypeScript IndexProgress interface | `src/types/file.ts` |
| 2026-01-22 | Updated progress component UI | `src/components/indexation-progress/indexation-progress.tsx` |
| 2026-01-22 | Updated test files for new field names | `src/stores/file-store.test.ts`, `src/components/indexation-progress/indexation-progress.test.tsx` |
| 2026-01-22 | **Code Review Fixes:** AC6 toast with stats, percentage rounding, SQLite cancel command, test warnings | Multiple files |

### File List

**New Files:**
- `src-tauri/src/commands/sqlite_indexation.rs` - Async Tauri command for SQLite indexation
- `src-tauri/src/commands/progress_emitter.rs` - 1 Hz progress event emitter

**Modified Files:**
- `src-tauri/src/commands/mod.rs` - Added sqlite_indexation and progress_emitter modules
- `src-tauri/src/lib.rs` - Registered build_sqlite_index and cancel_sqlite_indexation commands
- `src-tauri/src/indexer/progress.rs` - Added entries_per_second and elapsed_seconds fields
- `src/types/file.ts` - Updated IndexProgress interface, added SqliteIndexMetadata
- `src/components/indexation-progress/indexation-progress.tsx` - Updated for new fields and UI
- `src/stores/file-store.test.ts` - Updated test data for new field names
- `src/components/indexation-progress/indexation-progress.test.tsx` - Updated test data for new field names

