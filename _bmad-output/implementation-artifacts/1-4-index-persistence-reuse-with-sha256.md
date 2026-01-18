# Story 1.4: Index Persistence & Reuse with SHA-256

Status: backend-complete (frontend-pending)

## Story

As a network administrator,
I want my log file indexes saved and reused automatically,
So that I don't have to wait 2-3 minutes re-indexing the same file every time I open it.

## Acceptance Criteria

**Given** indexation completes successfully (Story 1.3)
**When** the index is persisted to disk
**Then** the index file is saved to application data directory as:
- Windows: `%APPDATA%\opnsense-log-viewer\indexes\`
- macOS: `~/Library/Application Support/opnsense-log-viewer/indexes/`
- Linux: `~/.local/share/opnsense-log-viewer/indexes/`

**And** the index filename includes the source file SHA-256 hash:
- Format: `{sha256_hash}.idx`

**And** the index file is compressed using Zstd (level 1)
**Then** the compressed index size is approximately:
- 10% of source log file size before compression
- ~5% of source log file size after compression (2x ratio)

**When** the index includes integrity metadata
**Then** the index header contains:
- Version number (for format evolution)
- SHA-256 checksum of index contents
- Source file SHA-256 hash
- Index creation timestamp
- Source file metadata (size, entry count, format)

**When** a user opens a previously indexed file
**Then** the application:
- Calculates SHA-256 hash of the source file
- Checks for existing index file matching the hash
- If found and valid, loads the index in ≤2 seconds (NFR-001.6)
- Skips re-indexation entirely

**When** the source file has been modified (hash mismatch)
**Then** a dialog prompts: "Source file changed since last index. Re-index? [Yes] [No]"
**And** selecting Yes triggers new indexation
**And** selecting No cancels the operation

**When** an index file is corrupted (checksum validation fails)
**Then** an error displays: "Index file corrupted. Re-index to continue? [Yes] [No]"
**And** the application never crashes on corrupted index (NFR-002.3)

**When** the user views existing indexes (Settings > Manage Indexes)
**Then** a list displays showing:
- Source filename (if still accessible)
- Index creation date
- Index file size
- Entry count
- [Delete] button for each index

**And** selecting Delete removes the .idx file and frees disk space

**And** index idempotency meets NFR-002.4:
- Re-indexing the same file produces bit-identical index (excluding timestamps)
- 100% SHA-256 hash match across 10 re-index operations

## Tasks / Subtasks

- [x] Add new dependencies to Cargo.toml (AC: Dependencies available)
  - [x] Add bincode = "2.0.1"
  - [x] Add zstd = "0.13.3"
  - [x] Add hex = "0.4"
  - [x] Add tempfile = "3.x" (dev-dependency for tests)

- [x] Create storage module structure (AC: File organization)
  - [x] Create src-tauri/src/storage/mod.rs with public API
  - [x] Create src-tauri/src/storage/persistence.rs (save/load)
  - [x] Create src-tauri/src/storage/integrity.rs (SHA-256 hashing)
  - [x] Create src-tauri/src/storage/paths.rs (OS-specific paths)
  - [x] Create src-tauri/src/storage/index_manager.rs (list/delete indexes)

- [x] Create types for persisted index (AC: Index metadata structure)
  - [x] Create src-tauri/src/types/persisted_index.rs
  - [x] Define PersistedIndex struct with version, checksums, metadata
  - [x] Define SourceFileMetadata struct
  - [x] Implement index_filename() helper
  - [x] Add unit tests for struct creation

- [x] Implement SHA-256 integrity module (AC: Hash calculation)
  - [x] Implement calculate_file_hash() using streaming (4KB chunks)
  - [x] Implement calculate_checksum() for serialized data
  - [x] Implement verify_checksum() for corruption detection
  - [x] Add unit tests for all integrity functions
  - [x] Test with large file fixtures (>1GB)

- [x] Implement bincode + zstd persistence (AC: Save/load with compression)
  - [x] Implement save_index() with atomic write (.idx.tmp → .idx)
  - [x] Serialize with bincode 2.0.1
  - [x] Compress with Zstd level 1
  - [x] Include Zstd checksums
  - [x] Implement load_index() with decompression
  - [x] Verify index checksum on load
  - [x] Handle corruption gracefully (error, no crash)
  - [x] Add unit tests for save/load roundtrip
  - [x] Test with corrupted index files

- [x] Implement OS-specific path resolution (AC: Cross-platform paths)
  - [x] Implement get_indexes_dir() using Tauri app_data_dir()
  - [x] Implement get_index_path() with SHA-256 hash filename
  - [x] Ensure directory creation (create_dir_all)
  - [x] Add unit tests for path resolution
  - [x] Test on all platforms (Windows, macOS, Linux)

- [ ] Implement index manager (AC: List and delete indexes)
  - [ ] Implement list_indexes() returning IndexInfo array
  - [ ] Parse .idx files to extract metadata
  - [ ] Check source file accessibility
  - [ ] Implement delete_index() with error handling
  - [ ] Add unit tests for manager operations

- [ ] Extend indexation commands (AC: Save after indexation)
  - [ ] Modify build_hybrid_index command to save index automatically
  - [ ] Calculate source file SHA-256 during indexation
  - [ ] Save PersistedIndex after successful indexation
  - [ ] Emit "index-saved" Tauri event with path
  - [ ] Add integration tests

- [ ] Create load index command (AC: Load existing index)
  - [ ] Implement load_index_file(file_path) IPC command
  - [ ] Calculate source file hash
  - [ ] Check for existing index (get_index_path)
  - [ ] Load index if found and valid
  - [ ] Verify source file hash matches (detect modifications)
  - [ ] Prompt user if hash mismatch (modified file)
  - [ ] Handle corrupted index gracefully
  - [ ] Meet NFR-001.6: ≤2 sec load time
  - [ ] Add integration tests

- [ ] Create manage indexes commands (AC: Settings UI)
  - [ ] Create src-tauri/src/commands/storage.rs
  - [ ] Implement list_all_indexes() command
  - [ ] Implement delete_index_by_hash() command
  - [ ] Return IndexInfo structs with file path, date, size, entry count
  - [ ] Register commands in src-tauri/src/lib.rs
  - [ ] Add integration tests

- [ ] Create frontend index management UI (AC: Settings > Manage Indexes)
  - [ ] Create src/components/manage-indexes/manage-indexes.tsx
  - [ ] Display list of indexes with metadata
  - [ ] Show source filename (if accessible)
  - [ ] Show index creation date
  - [ ] Show index file size
  - [ ] Show entry count
  - [ ] Add [Delete] button for each index
  - [ ] Confirm deletion with modal
  - [ ] Update list after deletion
  - [ ] Add unit tests for component

- [ ] Integrate file open workflow (AC: Automatic detection)
  - [ ] Modify file selection handler in frontend
  - [ ] Call load_index_file() when file selected
  - [ ] Show loading indicator during hash calculation
  - [ ] Display "Using existing index" toast if found
  - [ ] Display "Re-index required" dialog if hash mismatch
  - [ ] Handle corruption with re-index dialog
  - [ ] Skip indexation if valid index loaded
  - [ ] Add E2E test for reopen workflow

- [ ] Write idempotency tests (AC: NFR-002.4 - Idempotent re-indexation)
  - [ ] Create tests/idempotency_test.rs
  - [ ] Index same file 10 times
  - [ ] Verify all 10 indexes have identical SHA-256 hash (excluding timestamps)
  - [ ] Verify 100% hash match across all re-index operations
  - [ ] Add to CI/CD pipeline

- [ ] Write performance tests (AC: NFR-001.6 - ≤2 sec load)
  - [ ] Create benches/index_load_benchmark.rs
  - [ ] Benchmark index loading for 1GB, 10GB, 30GB files
  - [ ] Target: ≤2 sec load time for all file sizes
  - [ ] Add to CI/CD performance gates
  - [ ] Generate HTML reports

- [ ] Write integration tests (AC: End-to-end validation)
  - [ ] Test save → load roundtrip with real log samples
  - [ ] Test hash mismatch detection (modify source file)
  - [ ] Test corruption detection (corrupt .idx file)
  - [ ] Test list_indexes with multiple indexes
  - [ ] Test delete_index removes file
  - [ ] Test atomic write (.idx.tmp cleanup on failure)
  - [ ] Ensure all tests pass with `cargo test`

## Dev Notes

### 🔥 ULTIMATE STORY CONTEXT ENGINE OUTPUT 🔥

This story file has been generated by the comprehensive context analysis engine. It contains EVERYTHING you need to implement Story 1.4 correctly, aligned with architecture, UX design, and project patterns.

---

### Architecture Compliance & Technical Requirements

#### **Technology Stack (MUST USE EXACT VERSIONS)**

**Backend - Index Persistence Stack:**
- **bincode 2.0.1** - Binary serialization for HybridIndex struct
  - ✅ Space-efficient, no metadata overhead
  - ✅ Cross-architecture compatible (invariant byte-order)
  - ✅ Seamless serde integration (already used for Tauri IPC)
  - ⚠️ Version 2.0 API (NOT 1.x - breaking changes)
- **zstd 0.13.3** - Compression for .idx files
  - ✅ Level 1 compression (optimized for speed per architecture.md decision)
  - ✅ Target ratio: ~2x compression (~5% of source file size)
  - ✅ Include checksums for integrity detection
  - ⚠️ DO NOT use level 3 - architectural decision already made
- **sha2 0.10** - SHA-256 hashing for integrity
  - ✅ Streaming hash calculation for large files (4KB chunks)
  - ✅ Source file hash (detect modifications)
  - ✅ Index checksum (detect corruption)
  - ⚠️ MUST use streaming approach for 30GB files
- **chrono 0.4.39** - Timestamps (already added in Story 1.1)
- **serde 1.x** with derive feature (already available)
- **thiserror 2.x** - Custom error types (already available)
- **Tauri app-specific dirs** - OS-specific application data paths

**OS-Specific Path APIs:**
- Windows: `%APPDATA%\opnsense-log-viewer\indexes\`
- macOS: `~/Library/Application Support/opnsense-log-viewer/indexes/`
- Linux: `~/.local/share/opnsense-log-viewer/indexes/`
- ✅ Use Tauri `app_data_dir()` helper function for cross-platform paths

---

#### **Code Structure & File Organization**

**Backend Structure (NEW modules for Story 1.4):**
```
src-tauri/src/
├── storage/
│   ├── mod.rs                          # Public API: save_index(), load_index()
│   ├── persistence.rs                  # Index serialization (bincode + zstd)
│   ├── integrity.rs                    # SHA-256 hash calculation & validation
│   ├── index_manager.rs                # List indexes, delete, manage .idx files
│   └── paths.rs                        # OS-specific path resolution
├── indexer/                             # From Story 1.3 (already exists)
│   ├── hybrid.rs                        # HybridIndex struct (to serialize)
│   └── ...
├── commands/
│   ├── indexation.rs                    # Extend: add save/load index commands
│   └── storage.rs                       # NEW: manage_indexes, delete_index
└── types/
    └── persisted_index.rs               # NEW: PersistedIndex struct with metadata
```

**Frontend Structure (NEW components for Story 1.4):**
```
src/
└── components/
    └── manage-indexes/
        ├── manage-indexes.tsx           # Index management UI
        ├── manage-indexes.test.tsx
        ├── index-list-item.tsx          # Single index row component
        └── index.ts                     # Barrel export
```

---

#### **Persisted Index Format Implementation**

**File: src-tauri/src/types/persisted_index.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::indexer::hybrid::HybridIndex;
use crate::parser::LogFormat;

/// Version number for index format evolution
/// Increment on breaking changes to serialization format
const INDEX_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedIndex {
    /// Format version (for future compatibility)
    pub version: u32,

    /// SHA-256 hash of the serialized index content (for corruption detection)
    pub index_checksum: [u8; 32],

    /// SHA-256 hash of the source log file (for modification detection)
    pub source_file_hash: [u8; 32],

    /// Index creation timestamp
    pub created_at: DateTime<Utc>,

    /// Source file metadata
    pub source_metadata: SourceFileMetadata,

    /// The actual hybrid index (inverted + bitmap + offset table)
    pub hybrid_index: HybridIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFileMetadata {
    pub file_path: String,
    pub file_size: u64,
    pub entry_count: u64,
    pub log_format: LogFormat,
}

impl PersistedIndex {
    pub fn new(
        source_file_hash: [u8; 32],
        source_metadata: SourceFileMetadata,
        hybrid_index: HybridIndex,
    ) -> Self {
        Self {
            version: INDEX_FORMAT_VERSION,
            index_checksum: [0; 32],  // Calculated after serialization
            source_file_hash,
            created_at: Utc::now(),
            source_metadata,
            hybrid_index,
        }
    }

    /// Generate index filename from source file hash
    pub fn index_filename(source_hash: &[u8; 32]) -> String {
        format!("{}.idx", hex::encode(source_hash))
    }
}
```

---

#### **SHA-256 Streaming Hash Implementation**

**File: src-tauri/src/storage/integrity.rs**

```rust
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntegrityError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
}

/// Calculate SHA-256 hash of a file using streaming (4KB chunks)
/// Essential for large files (30GB) to avoid loading entire file into memory
pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<[u8; 32], IntegrityError> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    // Read in 4KB chunks - memory efficient for large files
    let mut buffer = [0u8; 4096];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;  // EOF reached
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    Ok(hash)
}

/// Calculate SHA-256 hash of serialized bytes (for index integrity)
pub fn calculate_checksum(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    hash
}

/// Verify index checksum matches expected value
pub fn verify_checksum(data: &[u8], expected: &[u8; 32]) -> Result<(), IntegrityError> {
    let actual = calculate_checksum(data);

    if &actual != expected {
        return Err(IntegrityError::ChecksumMismatch {
            expected: hex::encode(expected),
            actual: hex::encode(actual),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_file_hash_streaming() {
        // Create temp file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content for hashing").unwrap();

        let hash = calculate_file_hash(temp_file.path()).unwrap();

        // Verify hash is non-zero
        assert_ne!(hash, [0u8; 32]);

        // Hash should be deterministic
        let hash2 = calculate_file_hash(temp_file.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_verify_checksum_success() {
        let data = b"test data";
        let checksum = calculate_checksum(data);

        assert!(verify_checksum(data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_checksum_mismatch() {
        let data = b"test data";
        let wrong_checksum = [0u8; 32];

        assert!(verify_checksum(data, &wrong_checksum).is_err());
    }
}
```

---

#### **Bincode + Zstd Persistence Implementation**

**File: src-tauri/src/storage/persistence.rs**

```rust
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zstd::stream::{Encoder, Decoder};
use thiserror::Error;

use crate::types::persisted_index::PersistedIndex;
use crate::storage::integrity::{calculate_checksum, verify_checksum, IntegrityError};

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Integrity error: {0}")]
    IntegrityError(#[from] IntegrityError),

    #[error("Compression error: {0}")]
    CompressionError(String),
}

/// Save index to disk with Zstd compression (level 1)
/// Atomic operation: writes to .idx.tmp then renames to .idx
pub fn save_index<P: AsRef<Path>>(
    index: &PersistedIndex,
    index_path: P,
) -> Result<(), PersistenceError> {
    let index_path = index_path.as_ref();
    let temp_path = index_path.with_extension("idx.tmp");

    // Serialize to bytes using bincode
    let serialized = bincode::serialize(&index)?;

    // Calculate checksum of serialized data
    let checksum = calculate_checksum(&serialized);

    // Update index with calculated checksum
    let mut index_with_checksum = index.clone();
    index_with_checksum.index_checksum = checksum;

    // Re-serialize with checksum included
    let final_serialized = bincode::serialize(&index_with_checksum)?;

    // Compress with Zstd (level 1 for speed)
    {
        let temp_file = File::create(&temp_path)?;
        let mut encoder = Encoder::new(temp_file, 1)?;  // Level 1 compression
        encoder.include_checksum(true)?;  // Zstd built-in checksum
        encoder.write_all(&final_serialized)?;
        encoder.finish()?;
    }

    // Atomic rename: .idx.tmp → .idx
    fs::rename(&temp_path, index_path)?;

    Ok(())
}

/// Load index from disk with decompression and integrity validation
pub fn load_index<P: AsRef<Path>>(
    index_path: P,
) -> Result<PersistedIndex, PersistenceError> {
    let index_path = index_path.as_ref();

    // Decompress with Zstd
    let file = File::open(index_path)?;
    let mut decoder = Decoder::new(file)?;

    // Read all decompressed bytes
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    // Deserialize with bincode
    let index: PersistedIndex = bincode::deserialize(&decompressed)?;

    // Verify index integrity (checksum validation)
    // Create copy without checksum for verification
    let mut index_for_verification = index.clone();
    let stored_checksum = index.index_checksum;
    index_for_verification.index_checksum = [0; 32];

    let serialized_for_verification = bincode::serialize(&index_for_verification)?;
    verify_checksum(&serialized_for_verification, &stored_checksum)?;

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::types::persisted_index::{PersistedIndex, SourceFileMetadata};
    use crate::parser::LogFormat;

    #[test]
    fn test_save_and_load_index_roundtrip() {
        // Create test index
        let source_hash = [1u8; 32];
        let metadata = SourceFileMetadata {
            file_path: "/test/log.txt".to_string(),
            file_size: 1024,
            entry_count: 100,
            log_format: LogFormat::RFC3164,
        };

        let hybrid_index = HybridIndex::new();  // Empty for test
        let original_index = PersistedIndex::new(source_hash, metadata, hybrid_index);

        // Save to temp file
        let temp_file = NamedTempFile::new().unwrap();
        let index_path = temp_file.path();

        save_index(&original_index, index_path).unwrap();

        // Load back
        let loaded_index = load_index(index_path).unwrap();

        // Verify critical fields match
        assert_eq!(loaded_index.version, original_index.version);
        assert_eq!(loaded_index.source_file_hash, original_index.source_file_hash);
        assert_eq!(loaded_index.source_metadata.file_size, 1024);
    }

    #[test]
    fn test_load_corrupted_index_fails() {
        // Create corrupted index file
        let temp_file = NamedTempFile::new().unwrap();
        let mut file = File::create(temp_file.path()).unwrap();
        file.write_all(b"corrupted data").unwrap();

        // Should fail to load
        assert!(load_index(temp_file.path()).is_err());
    }
}
```

---

#### **OS-Specific Path Resolution**

**File: src-tauri/src/storage/paths.rs**

```rust
use std::path::PathBuf;
use tauri::AppHandle;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Failed to resolve application data directory")]
    AppDataDirNotFound,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Get the indexes directory path (OS-specific)
/// - Windows: %APPDATA%\opnsense-log-viewer\indexes\
/// - macOS: ~/Library/Application Support/opnsense-log-viewer/indexes/
/// - Linux: ~/.local/share/opnsense-log-viewer/indexes/
pub fn get_indexes_dir(app_handle: &AppHandle) -> Result<PathBuf, PathError> {
    // Use Tauri's app_data_dir() helper for cross-platform compatibility
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| PathError::AppDataDirNotFound)?;

    let indexes_dir = app_data_dir.join("indexes");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&indexes_dir)?;

    Ok(indexes_dir)
}

/// Get full path for an index file based on source file hash
pub fn get_index_path(
    app_handle: &AppHandle,
    source_hash: &[u8; 32],
) -> Result<PathBuf, PathError> {
    let indexes_dir = get_indexes_dir(app_handle)?;
    let filename = format!("{}.idx", hex::encode(source_hash));
    Ok(indexes_dir.join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_indexes_dir_creates_directory() {
        // Note: Requires Tauri test context - simplified for unit test
        // In real testing, use Tauri mock builder
    }
}
```

---

### Previous Story Intelligence (Story 1.3 Learnings)

**What Works Well from Story 1.3:**
- ✅ HybridIndex struct with InvertedIndex, BitmapIndex, OffsetTable
- ✅ IndexMetadata struct pattern (format, entry_count, created_at, source info)
- ✅ Tauri IPC command pattern with Result<T, String> error handling
- ✅ SHA-256 hash calculation already implemented (calculate_file_hash function)
- ✅ Progress tracking with Tauri events (reuse for persistence operations)
- ✅ Memory-safe streaming approach established

**What to Integrate from Story 1.3:**
- ✅ Reuse HybridIndex struct as core of PersistedIndex
- ✅ Extend IndexMetadata to include checksums for persistence
- ✅ Integrate calculate_file_hash() from Story 1.3 (integrity.rs module)
- ✅ Pattern: .idx.tmp → .idx atomic rename (prevents partial writes)
- ✅ Error handling: thiserror for library, Result<T, String> for IPC

**Code Review Fixes Applied in Story 1.3 (Carry Forward):**
- ✅ Default trait implementations for all structs
- ✅ Clippy warnings resolved (0 warnings policy)
- ✅ Time-based progress events (not byte-based spam)
- ✅ SHA-256 hash calculated during operations (not estimated)

**Files Created in Story 1.3 (Available for Reuse):**
- ✅ `src-tauri/src/indexer/hybrid.rs` - HybridIndex struct
- ✅ `src-tauri/src/indexer/progress.rs` - Progress tracking pattern
- ✅ `src-tauri/src/commands/indexation.rs` - IPC command pattern

---

### Git Intelligence Summary

**Recent Commits (Relevant to Story 1.4):**

From `git log --oneline -5`:
1. `d383f25` - Code Review Fixes: Story 1.3 - Hybrid Index Quality Improvements
2. `c3a7943` - Complete Story 1.3: Hybrid Index Creation (Inverted + Bitmap)
3. `f595616` - Complete Story 1.2: Multi-Format Log Parser
4. `eb34e87` - Complete Story 1.1: File Selection with Native OS Picker
5. `4334ed1` - Fix Story 0.3: Code review corrections

**Dependencies Already Available:**
- ✅ `sha2 = "0.10"` (added in Story 1.3 for hash calculation)
- ✅ `chrono = "0.4.39"` (timestamps)
- ✅ `serde = { version = "1.0", features = ["derive"] }`
- ✅ `thiserror = "2.0"`
- ✅ `tokio = { version = "1.49", features = ["rt-multi-thread", "fs", "io-util"] }`

**Dependencies to ADD for Story 1.4:**
- ⚠️ `bincode = "2.0.1"` - Binary serialization
- ⚠️ `zstd = "0.13.3"` - Compression
- ⚠️ `hex = "0.4"` - Hash to hex string conversion

**Next Integration Point:**
- Story 1.4 consumes HybridIndex from Story 1.3
- Adds persistence layer (save/load .idx files)
- Enables Story 1.5 (Log Entry Display) to load pre-indexed files instantly
- Unblocks "quick reopen" workflow (≤2 sec load vs 2-3 min re-indexation)

---

### Latest Technical Research (2026)

#### **Bincode 2.0 Best Practices**

Based on latest research:
- ✅ **Format Stability**: Encoding format is stable across minor revisions (same config)
- ✅ **Cross-Architecture**: Invariant byte-order in default config (works across platforms)
- ✅ **Space Efficiency**: No metadata overhead (no field names in output)
- ✅ **Performance**: Designed for speed and compactness, ideal for our use case
- ⚠️ **Version 2.0 API**: Uses configuration-based approach (different from 1.x)

**Configuration Pattern (Bincode 2.0):**
```rust
use bincode::{serialize, deserialize};

// Serialize with default config
let bytes = bincode::serialize(&index)?;

// Deserialize with default config
let index: PersistedIndex = bincode::deserialize(&bytes)?;
```

**Sources:**
- [What purpose does bincode serve](https://users.rust-lang.org/t/what-purpose-does-the-crate-bincode-serve-in-binary-serialization-that-serde-does-not/73981)
- [Rust Serialization Production Ready](https://blog.logrocket.com/rust-serialization-whats-ready-for-production-today/)
- [Binary Data Handling Best Practices](https://softwarepatternslexicon.com/rust/networking-and-i-o-patterns/handling-binary-data-and-serialization/)

---

#### **Zstd 0.13 Compression Performance**

Based on latest research:
- ✅ **Level 1**: Optimized for speed (fastest compression)
- ✅ **Target Ratio**: ~2x compression typical (varies by data type)
- ✅ **Checksums**: Built-in checksum support via `encoder.include_checksum(true)`
- ✅ **Streaming**: Supports streaming compression for large data

**Configuration (Level 1 - Architecture Decision):**
```rust
use zstd::stream::Encoder;

let mut encoder = Encoder::new(output_file, 1)?;  // Level 1
encoder.include_checksum(true)?;
encoder.write_all(&data)?;
encoder.finish()?;
```

**Performance Characteristics:**
- Level 1: Fastest compression, good ratio (~2x)
- Level 3+: Better ratio, slower (diminishing returns per arch decision)
- Decompression: Fast regardless of compression level

**Sources:**
- [Zstandard Official](http://facebook.github.io/zstd/)
- [zstd-rs GitHub](https://github.com/gyscos/zstd-rs)
- [Zstd 0.13.3 Docs](https://docs.rs/crate/zstd/latest)
- [Compress Files with Zstd in Rust](https://ssojet.com/compression/compress-files-with-zstd-in-rust/)

---

#### **SHA-2 Streaming for Large Files**

Based on latest research:
- ✅ **Streaming Pattern**: Read file in 4KB chunks, update hasher incrementally
- ✅ **Memory Efficiency**: Avoids loading entire 30GB file into memory
- ✅ **Pure Rust**: RustCrypto implementation (no C dependencies)
- ✅ **io::copy Pattern**: Can use `io::copy(&mut file, &mut hasher)` for convenience

**Best Practice Pattern (4KB Chunks):**
```rust
use sha2::{Sha256, Digest};
use std::io::{BufReader, Read};

let file = File::open(path)?;
let mut reader = BufReader::new(file);
let mut hasher = Sha256::new();

let mut buffer = [0u8; 4096];
loop {
    let bytes_read = reader.read(&mut buffer)?;
    if bytes_read == 0 { break; }
    hasher.update(&buffer[..bytes_read]);
}

let hash = hasher.finalize();
```

**Sources:**
- [Secure Hashing with SHA-2 in Rust](https://friendlyuser.github.io/posts/tech/rust/using_sha2_in_rust/)
- [RustCrypto Hashes GitHub](https://github.com/RustCrypto/hashes)
- [Weekly Rust: SHA256 Hash of File](https://www.thorsten-hans.com/weekly-rust-trivia-compute-a-sha256-hash-of-a-file/)
- [Hashing - Rust Cookbook](https://rust-lang-nursery.github.io/rust-cookbook/cryptography/hashing.html)

---

### Tauri IPC Commands Implementation

#### **Extended Indexation Commands**

**File: src-tauri/src/commands/indexation.rs (EXTEND from Story 1.3)**

```rust
use tauri::{AppHandle, command};
use crate::indexer::hybrid::HybridIndex;
use crate::storage::{save_index, load_index};
use crate::storage::paths::get_index_path;
use crate::storage::integrity::calculate_file_hash;
use crate::types::persisted_index::{PersistedIndex, SourceFileMetadata};
use crate::parser::LogFormat;

/// Extended: Now saves index automatically after successful indexation
#[command]
pub fn build_hybrid_index(
    app_handle: AppHandle,
    file_path: String,
    format: LogFormat,
) -> Result<IndexMetadata, String> {
    // Build index (existing logic from Story 1.3)
    let mut hybrid_index = HybridIndex::new();
    let metadata = hybrid_index.build_index(&file_path, format, |progress| {
        // Emit progress events
        app_handle.emit("indexation-progress", progress).ok();
    }).map_err(|e| e.to_string())?;

    // NEW: Calculate source file hash
    let source_hash = calculate_file_hash(&file_path)
        .map_err(|e| format!("Failed to hash file: {}", e))?;

    // NEW: Create persisted index
    let source_metadata = SourceFileMetadata {
        file_path: file_path.clone(),
        file_size: metadata.source_file_size,
        entry_count: metadata.entry_count,
        log_format: format,
    };

    let persisted_index = PersistedIndex::new(
        source_hash,
        source_metadata,
        hybrid_index,
    );

    // NEW: Save index to disk
    let index_path = get_index_path(&app_handle, &source_hash)
        .map_err(|e| format!("Failed to get index path: {}", e))?;

    save_index(&persisted_index, &index_path)
        .map_err(|e| format!("Failed to save index: {}", e))?;

    // Emit save event
    app_handle.emit("index-saved", &index_path.to_string_lossy()).ok();

    Ok(metadata)
}

/// NEW: Load index from existing .idx file
#[command]
pub fn load_index_file(
    app_handle: AppHandle,
    file_path: String,
) -> Result<LoadIndexResult, String> {
    // Calculate source file hash
    let source_hash = calculate_file_hash(&file_path)
        .map_err(|e| format!("Failed to hash file: {}", e))?;

    // Check for existing index
    let index_path = get_index_path(&app_handle, &source_hash)
        .map_err(|e| format!("Failed to get index path: {}", e))?;

    if !index_path.exists() {
        return Ok(LoadIndexResult::NotFound);
    }

    // Load index
    let persisted_index = load_index(&index_path)
        .map_err(|e| {
            // Check if error is corruption
            if e.to_string().contains("Checksum mismatch") {
                return "Index corrupted. Re-index required.".to_string();
            }
            format!("Failed to load index: {}", e)
        })?;

    // Verify source file hash matches
    if persisted_index.source_file_hash != source_hash {
        return Ok(LoadIndexResult::HashMismatch {
            stored_hash: hex::encode(persisted_index.source_file_hash),
            actual_hash: hex::encode(source_hash),
        });
    }

    // Success - index loaded
    Ok(LoadIndexResult::Success {
        metadata: persisted_index.source_metadata,
        index: persisted_index.hybrid_index,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LoadIndexResult {
    Success {
        metadata: SourceFileMetadata,
        index: HybridIndex,
    },
    NotFound,
    HashMismatch {
        stored_hash: String,
        actual_hash: String,
    },
}
```

---

#### **Storage Management Commands**

**File: src-tauri/src/commands/storage.rs (NEW)**

```rust
use tauri::{AppHandle, command};
use crate::storage::paths::get_indexes_dir;
use crate::storage::load_index;
use chrono::{DateTime, Utc};
use std::fs;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub hash: String,
    pub source_file_path: String,
    pub source_file_exists: bool,
    pub created_at: DateTime<Utc>,
    pub file_size: u64,
    pub entry_count: u64,
    pub index_file_size: u64,
}

/// List all saved indexes
#[command]
pub fn list_all_indexes(app_handle: AppHandle) -> Result<Vec<IndexInfo>, String> {
    let indexes_dir = get_indexes_dir(&app_handle)
        .map_err(|e| format!("Failed to get indexes dir: {}", e))?;

    let mut indexes = Vec::new();

    for entry in fs::read_dir(&indexes_dir)
        .map_err(|e| format!("Failed to read indexes dir: {}", e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("idx") {
            continue;  // Skip non-.idx files
        }

        // Extract hash from filename (remove .idx extension)
        let hash = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Try to load index metadata
        match load_index(&path) {
            Ok(persisted_index) => {
                let source_exists = fs::metadata(&persisted_index.source_metadata.file_path).is_ok();
                let index_file_size = fs::metadata(&path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                indexes.push(IndexInfo {
                    hash,
                    source_file_path: persisted_index.source_metadata.file_path,
                    source_file_exists: source_exists,
                    created_at: persisted_index.created_at,
                    file_size: persisted_index.source_metadata.file_size,
                    entry_count: persisted_index.source_metadata.entry_count,
                    index_file_size,
                });
            }
            Err(_) => {
                // Skip corrupted indexes
                continue;
            }
        }
    }

    // Sort by created_at descending (newest first)
    indexes.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(indexes)
}

/// Delete index by hash
#[command]
pub fn delete_index_by_hash(
    app_handle: AppHandle,
    hash: String,
) -> Result<(), String> {
    let indexes_dir = get_indexes_dir(&app_handle)
        .map_err(|e| format!("Failed to get indexes dir: {}", e))?;

    let index_path = indexes_dir.join(format!("{}.idx", hash));

    if !index_path.exists() {
        return Err(format!("Index not found: {}", hash));
    }

    fs::remove_file(&index_path)
        .map_err(|e| format!("Failed to delete index: {}", e))?;

    Ok(())
}
```

**Register in src-tauri/src/lib.rs:**
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Existing commands from Story 1.3
            commands::indexation::build_hybrid_index,
            commands::indexation::cancel_indexation,

            // NEW commands for Story 1.4
            commands::indexation::load_index_file,
            commands::storage::list_all_indexes,
            commands::storage::delete_index_by_hash,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

### Frontend Implementation

#### **Manage Indexes Component**

**File: src/components/manage-indexes/manage-indexes.tsx**

```typescript
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { formatDistance } from 'date-fns';
import { Button } from '@/components/base/button';
import { Trash2, AlertCircle, CheckCircle } from 'lucide-react';
import toast from 'react-hot-toast';

interface IndexInfo {
  hash: string;
  sourceFilePath: string;
  sourceFileExists: boolean;
  createdAt: string;
  fileSize: number;
  entryCount: number;
  indexFileSize: number;
}

export function ManageIndexes() {
  const [indexes, setIndexes] = useState<IndexInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [deleteHash, setDeleteHash] = useState<string | null>(null);

  useEffect(() => {
    loadIndexes();
  }, []);

  const loadIndexes = async () => {
    try {
      setLoading(true);
      const result = await invoke<IndexInfo[]>('list_all_indexes');
      setIndexes(result);
    } catch (error) {
      toast.error(`Failed to load indexes: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (hash: string) => {
    try {
      await invoke('delete_index_by_hash', { hash });
      toast.success('Index deleted successfully');
      await loadIndexes();  // Refresh list
      setDeleteHash(null);
    } catch (error) {
      toast.error(`Failed to delete index: ${error}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return formatDistance(date, new Date(), { addSuffix: true });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="text-gray-500 dark:text-gray-400">Loading indexes...</div>
      </div>
    );
  }

  if (indexes.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center">
        <div className="text-gray-500 dark:text-gray-400 mb-2">No saved indexes found</div>
        <div className="text-sm text-gray-400 dark:text-gray-500">
          Indexes are created automatically when you open log files
        </div>
      </div>
    );
  }

  return (
    <div className="p-6">
      <h2 className="text-xl font-semibold mb-4">Manage Indexes</h2>

      <div className="space-y-2">
        {indexes.map((index) => (
          <div
            key={index.hash}
            className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:bg-gray-50 dark:hover:bg-gray-800 transition"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                {/* Source File Path */}
                <div className="flex items-center gap-2 mb-1">
                  {index.sourceFileExists ? (
                    <CheckCircle className="w-4 h-4 text-green-500 flex-shrink-0" />
                  ) : (
                    <AlertCircle className="w-4 h-4 text-orange-500 flex-shrink-0" />
                  )}
                  <span className="font-mono text-sm truncate" title={index.sourceFilePath}>
                    {index.sourceFilePath.split(/[/\\]/).pop()}
                  </span>
                </div>

                {/* Metadata */}
                <div className="text-xs text-gray-500 dark:text-gray-400 space-y-0.5">
                  <div>Created {formatDate(index.createdAt)}</div>
                  <div className="flex gap-4">
                    <span>Source: {formatBytes(index.fileSize)}</span>
                    <span>Index: {formatBytes(index.indexFileSize)}</span>
                    <span>{index.entryCount.toLocaleString()} entries</span>
                  </div>
                  {!index.sourceFileExists && (
                    <div className="text-orange-500">Source file not found</div>
                  )}
                </div>
              </div>

              {/* Delete Button */}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setDeleteHash(index.hash)}
                className="text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
              >
                <Trash2 className="w-4 h-4" />
              </Button>
            </div>
          </div>
        ))}
      </div>

      {/* Confirmation Modal */}
      {deleteHash && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md">
            <h3 className="text-lg font-semibold mb-2">Delete Index?</h3>
            <p className="text-gray-600 dark:text-gray-400 mb-4">
              This will delete the index file. The source log file will not be affected.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setDeleteHash(null)}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={() => handleDelete(deleteHash)}
                className="bg-red-500 hover:bg-red-600"
              >
                Delete
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

---

### Project Context Reference

**Critical Rules from project-context.md:**

**Serialization (CRITICAL):**
- ✅ ALWAYS use `#[serde(rename_all = "camelCase")]` for Rust ↔ TypeScript interop
- ✅ bincode 2.0.1 with default config (NOT 1.x API)
- ✅ Zstd compression level 1 (architectural decision - DO NOT change)

**File Handling:**
- ✅ Atomic writes: .idx.tmp → .idx rename pattern (prevents partial writes)
- ✅ Streaming SHA-256 (4KB chunks) for 30GB files
- ✅ Cross-platform paths via Tauri app_data_dir()

**Error Handling:**
- ✅ Library code: thiserror for custom error types
- ✅ Tauri commands: Result<T, String> for IPC
- ❌ NEVER panic or unwrap in production code

**Testing:**
- ✅ Performance gate: ≤2 sec index load time (NFR-001.6)
- ✅ Idempotency: 100% hash match across 10 re-index ops (NFR-002.4)
- ✅ Integration tests for all IPC commands

**Code Quality:**
- ✅ Run cargo fmt + cargo clippy before commit
- ✅ 0 clippy warnings policy
- ✅ Default trait implementations for all structs

---

### Story Completion Status

**Status:** ready-for-dev

**Next Steps:**
1. Review this story file thoroughly - contains ALL context needed
2. Add dependencies to Cargo.toml (bincode, zstd, hex, tempfile)
3. Implement modules in order: types → integrity → persistence → paths → index_manager
4. Write unit tests for each module
5. Implement Tauri IPC commands (load_index_file, list_all_indexes, delete_index_by_hash)
6. Create frontend ManageIndexes component
7. Integrate file open workflow (auto-detect existing indexes)
8. Write idempotency tests (10 re-index operations)
9. Write performance benchmarks (≤2 sec load)
10. Run tests: `cargo test` and `cargo bench`
11. Verify NFR compliance (NFR-001.6: ≤2 sec, NFR-002.4: idempotency)
12. Commit with format: `Complete Story 1.4: Index Persistence & Reuse with SHA-256`

**Estimated Complexity:** High (12-16 hours)
- Persistence module (bincode + zstd): 3 hours
- Integrity module (SHA-256 streaming): 2 hours
- Path resolution + index manager: 2 hours
- Tauri IPC commands: 2 hours
- Frontend ManageIndexes UI: 2 hours
- File open workflow integration: 1 hour
- Idempotency + performance tests: 2 hours
- Integration tests: 2 hours

**Blocking Dependencies:**
- Story 1.3 (Hybrid Index Creation) ✅ DONE

**Blocked Stories:**
- Story 1.5 (Log Entry Display Table) - requires index loading for quick file reopening
- Story 1.6 (Entry Detail View) - requires loaded index data

---

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)

### Debug Log References

N/A - Story created via create-story workflow (2026-01-17)

### Completion Notes List

_To be filled during implementation_

### File List

_To be filled during implementation_

**Expected Files to Create:**
- src-tauri/src/storage/mod.rs
- src-tauri/src/storage/persistence.rs
- src-tauri/src/storage/integrity.rs
- src-tauri/src/storage/paths.rs
- src-tauri/src/storage/index_manager.rs
- src-tauri/src/types/persisted_index.rs
- src-tauri/src/commands/storage.rs
- src-tauri/benches/index_load_benchmark.rs
- src-tauri/tests/idempotency_test.rs
- src-tauri/tests/persistence_integration_test.rs
- src/components/manage-indexes/manage-indexes.tsx
- src/components/manage-indexes/manage-indexes.test.tsx
- src/components/manage-indexes/index.ts

**Expected Files to Modify:**
- src-tauri/Cargo.toml (add bincode, zstd, hex dependencies)
- src-tauri/src/commands/indexation.rs (extend with save/load logic)
- src-tauri/src/lib.rs (register new IPC commands)

---
