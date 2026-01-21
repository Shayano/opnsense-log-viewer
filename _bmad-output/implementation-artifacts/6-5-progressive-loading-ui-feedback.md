# Story 6.5: Progressive Loading UI Feedback

Status: done

## Story

As a network administrator,
I want clear visual feedback during progressive loading,
So that I understand indexation progress and know when I can start filtering.

## Problem Statement

**Current Behavior:**
- IndexationProgress component shows basic progress (bytes, speed, ETA)
- No indication of batch-based progressive indexation progress
- No "partial filtering available" notification when first batch completes
- No cache hit feedback (Story 6.4 events not displayed to user)
- FileState store lacks progressive loading state fields

**Target Behavior (per Tech Spec Tasks 16-18):**
- Progress display shows batch progress: "Batch 6 / 14"
- Progress display shows entries: "32M / 71M entries"
- "Partial filtering available" badge appears when `partial_filter_available: true`
- Toast notification on cache hit: "Index loaded from cache"
- FileState extended with `indexProgress`, `partialFilterAvailable`, `cacheHit`
- Follows existing Tailwind patterns with dark/light theme support

## Acceptance Criteria

### AC1: Enhanced Progress Display
**Given** progressive indexation is running
**When** each batch completes
**Then** the UI updates with:
- Percentage: "45%"
- Entries: "32M / 71M entries"
- Batches: "Batch 6 / 14"
- Speed and ETA (existing)

### AC2: Partial Filtering Badge
**Given** the first batch completes during indexation
**When** `partial_filter_available` becomes `true` in progress event
**Then** a badge appears: "Partial filtering available"
**And** the badge is visually prominent (green indicator)
**And** the user understands they can start filtering before indexation completes

### AC3: Cache Hit Toast Notification
**Given** a file loads from cache (Story 6.4)
**When** the "index-cache-hit" event fires
**Then** a toast notification appears: "Index loaded from cache"
**And** the progress dialog shows "Loaded from cache (instant)" instead of normal progress

### AC4: Cache Miss Event Handling
**Given** a cache miss occurs
**When** the "index-cache-miss" event fires
**Then** normal progressive indexation begins
**And** no misleading "loading from cache" UI is shown

### AC5: FileState Store Extension
**Given** FileState in Zustand store exists
**When** progressive loading state is needed
**Then** FileState includes:
- `indexProgress: IndexProgress | null`
- `partialFilterAvailable: boolean`
- `cacheHit: boolean`
**And** appropriate actions to update these fields

### AC6: Theme Support
**Given** the app supports dark/light themes
**When** progressive loading UI is displayed
**Then** all new UI elements follow existing Tailwind patterns
**And** work correctly in both themes

## Tasks / Subtasks

- [x] Task 16: Extend FileState for progressive loading (AC: #5)
  - [x] 16.1: Add `indexProgress: IndexProgress | null` to FileState interface
  - [x] 16.2: Add `partialFilterAvailable: boolean` to FileState interface
  - [x] 16.3: Add `cacheHit: boolean` to FileState interface
  - [x] 16.4: Add `setIndexProgress(progress: IndexProgress | null)` action
  - [x] 16.5: Add `setPartialFilterAvailable(available: boolean)` action
  - [x] 16.6: Add `setCacheHit(hit: boolean)` action
  - [x] 16.7: Add typed selectors: `useIndexProgress()`, `usePartialFilterAvailable()`, `useCacheHit()`
  - [x] 16.8: Update `clearFile()` action to reset new fields
  - [x] 16.9: Add unit tests for new store fields and actions

- [x] Task 17: Update progress component for partial availability (AC: #1, #2, #6)
  - [x] 17.1: Update `IndexProgress` interface to include new fields from backend:
    - `entriesProcessed: number`
    - `totalEntriesEstimate: number`
    - `currentBatch: number`
    - `totalBatches: number`
    - `partialFilterAvailable: boolean`
  - [x] 17.2: Add entries progress display: "{entriesProcessed} / {totalEntriesEstimate} entries"
  - [x] 17.3: Add batch progress display: "Batch {currentBatch} / {totalBatches}"
  - [x] 17.4: Create "Partial filtering available" badge component
    - Green background with checkmark icon
    - Appears when `partialFilterAvailable` is true
    - Tooltip: "You can start filtering now with partial results"
  - [x] 17.5: Add cache hit display mode: "Loaded from cache (instant)"
    - Different UI state when `cacheHit` is true
    - Skip normal progress display, show success state immediately
  - [x] 17.6: Ensure dark/light theme compatibility for all new elements
  - [x] 17.7: Add unit tests for new progress display elements

- [x] Task 18: Add cache event listeners (AC: #3, #4)
  - [x] 18.1: Add listener for "index-cache-hit" event in IndexationProgress component
    - Extract event payload: `{ filePath, cacheHit, cacheAgeSeconds }`
    - Call `setCacheHit(true)` on file store
    - Display toast: "Index loaded from cache"
  - [x] 18.2: Add listener for "index-cache-miss" event
    - Call `setCacheHit(false)` on file store
    - Optional: show brief "Building index..." message
  - [x] 18.3: Update existing progress event listener to extract `partialFilterAvailable`
    - Update FileState via `setPartialFilterAvailable()`
  - [x] 18.4: Add cleanup for new event listeners in useEffect cleanup
  - [x] 18.5: Add unit tests for cache event handling

## Dev Notes

### Critical Architecture Patterns

**From project-context.md (MANDATORY):**
- ✅ TypeScript strict mode REQUIRED
- ✅ Boolean props: `is/has/can` prefix (e.g., `isLoading`, `hasError`)
- ✅ Event handlers: `handle` + event (e.g., `handleCacheHit`)
- ✅ Zustand selective subscriptions: `useStore((state) => state.field)`
- ✅ Tauri events: kebab-case (e.g., "index-cache-hit")
- ✅ Toast notifications via react-hot-toast

**Zustand Pattern (from existing file-store.ts):**
```typescript
// Selective subscriptions
const indexProgress = useFileStore((state) => state.indexProgress);

// Typed selectors for components
export const useIndexProgress = () => useFileStore((state) => state.indexProgress);

// Action pattern
setIndexProgress: (progress) => set({ indexProgress: progress }),
```

**Tauri Event Listener Pattern (from App.tsx):**
```typescript
useEffect(() => {
  const unlistenPromise = listen<EventPayload>('event-name', (event) => {
    // Handle event
    console.log('Event received:', event.payload);
  });

  return () => {
    unlistenPromise.then((unlisten) => unlisten());
  };
}, []);
```

**Toast Pattern (from existing code):**
```typescript
import toast from 'react-hot-toast';

// Success toast
toast.success('Index loaded from cache');

// Info toast with icon
toast('Loading index...', { icon: '⏳' });
```

### Backend Event Payloads (from Story 6.4)

**IndexCacheEvent (from cache.rs):**
```typescript
interface IndexCacheEvent {
  filePath: string;
  cacheHit: boolean;
  cacheAgeSeconds: number | null;
  reason: string | null;
}
```

**Extended IndexProgress (from Story 6.3 progressive.rs):**
```typescript
interface IndexProgress {
  percentage: number;
  bytesProcessed: number;
  totalBytes: number;
  speedGbps: number;
  etaSeconds: number;
  // New fields from Story 6.3:
  entriesProcessed: number;
  totalEntriesEstimate: number;
  currentBatch: number;
  totalBatches: number;
  partialFilterAvailable: boolean;
}
```

### UI Component Patterns

**Progress Badge Pattern:**
```tsx
{partialFilterAvailable && (
  <div className="flex items-center gap-2 px-3 py-1.5 bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300 rounded-full text-sm font-medium">
    <CheckCircle className="w-4 h-4" />
    Partial filtering available
  </div>
)}
```

**Cache Hit Display Pattern:**
```tsx
{cacheHit ? (
  <div className="text-center py-8">
    <CheckCircle className="w-12 h-12 text-green-500 mx-auto mb-3" />
    <p className="text-lg font-medium">Loaded from cache</p>
    <p className="text-sm text-gray-500 dark:text-gray-400">Instant load</p>
  </div>
) : (
  // Normal progress UI
)}
```

### Source File Locations and Key Lines

| File | Purpose | Key Lines |
|------|---------|-----------|
| `src/stores/file-store.ts` | Extend for progressive state | Lines 7-28 (interface), 33-44 (actions) |
| `src/components/indexation-progress/indexation-progress.tsx` | Update progress UI | Lines 8-14 (interface), 79-112 (render) |
| `src/types/file.ts` | Type definitions if needed | Consider adding IndexProgress type |
| `src/App.tsx` | Event listener patterns | Lines 109-122 (listen pattern) |

### Previous Story Intelligence (6.4)

**Patterns established in Story 6.4:**
- Cache events emitted: "index-cache-hit" and "index-cache-miss"
- `IndexCacheEvent` struct with camelCase serde for IPC
- Cache check happens in `build_hybrid_index()` before indexation
- Events include `cacheAgeSeconds` for UI display

**Key integration points:**
- Cache events fire BEFORE progress events (on hit: no progress events follow)
- Cache miss transitions to normal progressive indexation flow
- `partialFilterAvailable` comes from `IndexProgress` event (Story 6.3)

### Previous Story Intelligence (6.3)

**From Story 6.3 progressive indexation:**
- `IndexProgress` struct extended with batch tracking fields
- `partial_filter_available` becomes true after first batch completes
- Events emitted via `app.emit("indexation-progress", &progress)`
- Entry counts are estimates until indexation completes

### Git Intelligence

**Recent commits (relevant patterns):**
- `316121e` feat(Story 6.4): implement persistent index cache for instant reload
  - Added cache event emission in `commands/indexation.rs`
  - Pattern: `app.emit("index-cache-hit", IndexCacheEvent {...})`
- `b23f818` feat(Story 6.3): implement progressive indexation with early filtering
  - Extended `IndexProgress` with batch fields
  - Added `partial_filter_available` field

### Testing Requirements

**Unit Tests (in indexation-progress.test.tsx):**
```typescript
describe('IndexationProgress', () => {
  it('should display batch progress when available', () => {
    // Test batch display: "Batch 3 / 14"
  });

  it('should display entry progress when available', () => {
    // Test entries display: "32M / 71M entries"
  });

  it('should show partial filtering badge when available', () => {
    // Test badge appears with green styling
  });

  it('should show cache hit state correctly', () => {
    // Test "Loaded from cache (instant)" display
  });
});
```

**Store Tests (in file-store.test.ts - create if needed):**
```typescript
describe('FileStore progressive loading', () => {
  it('should update indexProgress', () => {
    const { setIndexProgress } = useFileStore.getState();
    setIndexProgress({ percentage: 50, ... });
    expect(useFileStore.getState().indexProgress?.percentage).toBe(50);
  });

  it('should reset progressive fields on clearFile', () => {
    // Test clearFile resets partialFilterAvailable, cacheHit
  });
});
```

### Project Structure Notes

**Files to modify:**
- `src/stores/file-store.ts` - Add progressive loading state
- `src/components/indexation-progress/indexation-progress.tsx` - Enhance UI
- `src/components/indexation-progress/indexation-progress.test.tsx` - Add tests

**Optional new files:**
- `src/stores/file-store.test.ts` - Store unit tests (if not exists)

**Module structure maintained:**
```
src/
├── stores/
│   └── file-store.ts          # Extend with progressive state
├── components/
│   └── indexation-progress/
│       ├── indexation-progress.tsx       # Enhance UI
│       └── indexation-progress.test.tsx  # Add tests
└── types/
    └── file.ts                # Type definitions (optional update)
```

### References

- [Source: tech-spec-hybrid-progressive-indexation.md#Phase 6: Frontend Progressive UI]
- [Source: tech-spec-hybrid-progressive-indexation.md#Tasks 16-18]
- [Source: epics.md#Story 6.5: Progressive Loading UI Feedback]
- [Source: project-context.md#Zustand State Management]
- [Source: 6-4-persistent-index-cache-for-instant-reload.md#Cache Event Pattern]
- [Source: 6-3-progressive-indexation-with-early-filtering.md#IndexProgress fields]

### Performance Targets

| Metric | Target |
|--------|--------|
| UI update frequency | Match backend event frequency (~1/sec) |
| Re-render efficiency | Only progress-dependent components re-render |
| Toast display timing | Immediate on cache hit event |

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Event type mismatch | Use TypeScript interfaces matching backend structs |
| Missing event cleanup | Use useEffect cleanup pattern from App.tsx |
| Re-render performance | Use Zustand selective subscriptions |
| Theme inconsistency | Use existing Tailwind dark: variants |
| Backend field changes | IndexProgress interface should match Story 6.3 backend |

### Architectural Notes

**Event Flow Diagram:**
```
Backend emits events
       │
       ├──► "index-cache-hit"
       │         │
       │         ▼
       │    Display toast
       │    Set cacheHit=true
       │    Show instant load UI
       │
       ├──► "index-cache-miss"
       │         │
       │         ▼
       │    Set cacheHit=false
       │    Prepare for progress events
       │
       └──► "indexation-progress"
                 │
                 ▼
           Update indexProgress state
           Check partialFilterAvailable
           Re-render progress component
```

**Component State Flow:**
```
IndexationProgress component
       │
       ├── Props: isIndexing, onComplete, onError
       │
       ├── Local state: progress (IndexProgress)
       │
       ├── Store state: cacheHit, partialFilterAvailable
       │
       └── Render states:
           ├── Cache hit → Show instant load success
           ├── Indexing + partial available → Show badge + progress
           └── Indexing → Show normal progress
```

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- TypeScript build: passes with 0 errors
- Unit tests: 30/30 passing (file-store.test.ts: 16, indexation-progress.test.tsx: 14)
- Pre-existing test failure in file-selector.test.tsx unrelated to Story 6.5 changes

### Completion Notes List

- ✅ Task 16: Extended FileState Zustand store with progressive loading fields
  - Added `indexProgress`, `partialFilterAvailable`, `cacheHit` state fields
  - Added `setIndexProgress`, `setPartialFilterAvailable`, `setCacheHit` actions
  - Added typed selectors for all new fields
  - Updated `clearFile()` to reset all progressive loading fields including `isLoading`
  - Created comprehensive unit tests (16 tests passing)

- ✅ Task 17: Enhanced IndexationProgress component UI
  - Added `IndexProgress` and `IndexCacheEvent` type definitions to `src/types/file.ts`
  - Added entries progress display with M/K formatting
  - Added batch progress display "Batch X / Y"
  - Added green "Partial filtering available" badge with CheckCircle icon and tooltip (AC2)
  - Added cache hit display mode with Zap icon showing instant load
  - All elements support dark/light themes via Tailwind CSS
  - Created comprehensive unit tests (14 tests passing)

- ✅ Task 18: Implemented cache event listeners
  - Added "index-cache-hit" listener with toast notification
  - Added "index-cache-miss" listener to reset cache state
  - Updated progress event listener to extract `partialFilterAvailable`
  - Proper useEffect cleanup for all event listeners (verified async cleanup in tests)
  - Tests verify event handling and store updates

### File List

**Modified:**
- `src/stores/file-store.ts` - Extended with progressive loading state (indexProgress, partialFilterAvailable, cacheHit)
- `src/types/file.ts` - Added IndexProgress and IndexCacheEvent interfaces
- `src/components/indexation-progress/indexation-progress.tsx` - Enhanced UI with batch/entry progress, cache hit display, partial filtering badge with tooltip

**Created:**
- `src/stores/file-store.test.ts` - Unit tests for file store progressive loading (16 tests)

**Updated:**
- `src/components/indexation-progress/indexation-progress.test.tsx` - Extended with Story 6.5 tests (14 tests)

### Change Log

- 2026-01-22: Story 6.5 implementation complete - Progressive Loading UI Feedback (Tasks 16-18)
- 2026-01-22: Code review fixes applied (4 MEDIUM issues fixed)

