# Story 6.1: SQLite Schema & Connection Infrastructure

Status: done

## Story

As a developer,
I want a SQLite database schema optimized for log storage and querying,
So that the application can efficiently store and query 70M+ entries.

## Acceptance Criteria

### AC1: rusqlite Dependency Added
**Given** the project needs SQLite support
**When** I check Cargo.toml
**Then** rusqlite is configured with required features:
```toml
rusqlite = { version = "0.32", features = ["bundled", "backup", "functions"] }
```

### AC2: Database Schema Created
**Given** rusqlite is available
**When** I create the database schema
**Then** the following tables exist:
```sql
-- Core log entries (denormalized for query performance)
CREATE TABLE entries (
    id INTEGER PRIMARY KEY,
    byte_offset INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    source_ip TEXT,
    source_port INTEGER,
    dest_ip TEXT,
    dest_port INTEGER,
    action TEXT NOT NULL,
    protocol TEXT,
    interface TEXT,
    rule_id TEXT,
    raw_line TEXT
);

-- File metadata
CREATE TABLE file_info (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    entry_count INTEGER DEFAULT 0,
    indexed_at TEXT NOT NULL,
    UNIQUE(file_hash)
);

-- Indexes for fast filtering
CREATE INDEX idx_source_ip ON entries(source_ip);
CREATE INDEX idx_dest_ip ON entries(dest_ip);
CREATE INDEX idx_action ON entries(action);
CREATE INDEX idx_protocol ON entries(protocol);
CREATE INDEX idx_interface ON entries(interface);
CREATE INDEX idx_timestamp ON entries(timestamp);
CREATE INDEX idx_ports ON entries(source_port, dest_port);
```

### AC3: PRAGMA Optimizations Applied
**Given** the database is opened
**When** I apply SQLite PRAGMA settings
**Then** the following optimizations are active:
```sql
PRAGMA journal_mode = WAL;        -- Concurrent reads during write
PRAGMA synchronous = NORMAL;      -- Safe for WAL, avoid fsync per commit
PRAGMA cache_size = -64000;       -- 64MB cache
PRAGMA mmap_size = 268435456;     -- 256MB memory-mapped I/O
PRAGMA temp_store = MEMORY;       -- Temp tables in memory
PRAGMA page_size = 4096;          -- Standard page size
```

### AC4: Connection Pool Implemented
**Given** a connection pool is needed for concurrent access
**When** I implement SqliteConnectionPool
**Then** it provides:
- Read-only connections for queries (multiple)
- Single write connection for inserts (exclusive)
- Connection recycling with health checks
- Graceful shutdown with pending commit flush

### AC5: Hash-Based Database Lookup
**Given** a new log file is opened
**When** I call get_or_create_database(file_path)
**Then** the database is stored at `{app_data}/log_indexes/{file_hash}.sqlite`
**And** if database already exists and hash matches, return existing connection
**And** if hash differs, delete old database and create new

## Tasks / Subtasks

- [x] Task 1: Add rusqlite dependency (AC: 1)
  - [x] 1.1 Add rusqlite to Cargo.toml with features: bundled, backup, functions
  - [x] 1.2 Verify compilation succeeds with new dependency

- [x] Task 2: Implement schema.rs (AC: 2)
  - [x] 2.1 Create `src-tauri/src/indexer/sqlite/schema.rs`
  - [x] 2.2 Define `create_schema(conn: &Connection) -> Result<()>`
  - [x] 2.3 Implement CREATE TABLE statements for entries and file_info
  - [x] 2.4 Implement CREATE INDEX statements for all filterable fields
  - [x] 2.5 Add unit tests verifying table and index creation

- [x] Task 3: Implement connection.rs with PRAGMA config (AC: 3)
  - [x] 3.1 Create `src-tauri/src/indexer/sqlite/connection.rs`
  - [x] 3.2 Define `configure_connection(conn: &Connection) -> Result<()>`
  - [x] 3.3 Apply all required PRAGMA settings
  - [x] 3.4 Add unit tests verifying PRAGMA values after configuration

- [x] Task 4: Implement pool.rs with read/write management (AC: 4)
  - [x] 4.1 Create `src-tauri/src/indexer/sqlite/pool.rs`
  - [x] 4.2 Define `SqliteConnectionPool` struct with write_conn and read_conns
  - [x] 4.3 Implement `get_write_connection()` with exclusive access
  - [x] 4.4 Implement `get_read_connection()` returning pooled read-only connection
  - [x] 4.5 Implement health check on connection reuse
  - [x] 4.6 Implement `shutdown()` with commit flush
  - [x] 4.7 Add unit tests for concurrent read access

- [x] Task 5: Implement cache.rs with hash-based lookup (AC: 5)
  - [x] 5.1 Create `src-tauri/src/indexer/sqlite/cache.rs`
  - [x] 5.2 Define `get_or_create_database(app_handle, file_path) -> Result<SqliteConnectionPool>`
  - [x] 5.3 Use SHA256-based file hash (first 1MB + last 1MB + size) for fast identification
  - [x] 5.4 Store databases in `{app_data}/log_indexes/{file_hash}.sqlite`
  - [x] 5.5 Implement hash validation for existing databases
  - [x] 5.6 Implement old database cleanup when hash differs
  - [x] 5.7 Add unit tests for hash calculation and database validation

- [x] Task 6: Create module structure (AC: all)
  - [x] 6.1 Create `src-tauri/src/indexer/sqlite/mod.rs`
  - [x] 6.2 Re-export public API: SqliteConnectionPool, create_schema, get_or_create_database

## Dev Notes

### Architecture Context

This story establishes the SQLite foundation replacing the rkyv/mmap hybrid approach that crashes at 72M entries. SQLite provides:
- **No 32-bit pointer limit** - Handles unlimited entries
- **Disk-based storage** - Bounded memory usage regardless of file size
- **WAL mode** - Concurrent reads during write, crash recovery
- **SQL queries** - Replaces custom bitmap/inverted index logic

### Critical Technical Requirements

**rusqlite Version:** Use 0.32.x with exact version pinning per project-context.md rules
```toml
rusqlite = { version = "0.32", features = ["bundled", "backup", "functions"] }
```
- `bundled` - Compiles SQLite from source (cross-platform, no system dependency)
- `backup` - Enables backup API for potential future use
- `functions` - Enables custom SQL functions (needed for Story 6.4 REGEXP support)

**Error Handling Pattern:**
```rust
// Library code in indexer/sqlite/
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SqliteError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

**Serde Interop:** Not needed for this story - SQLite internal only. Story 6.4 will handle LogEntry <-> SQL conversion.

### File Structure Requirements

Create new sqlite submodule in existing indexer module:
```
src-tauri/src/indexer/
├── mod.rs              # Add: pub mod sqlite;
├── sqlite/             # NEW - This story
│   ├── mod.rs          # Re-exports
│   ├── schema.rs       # CREATE TABLE/INDEX
│   ├── connection.rs   # PRAGMA configuration
│   ├── pool.rs         # Connection pool
│   └── cache.rs        # Hash-based DB lookup
├── bitmap.rs           # Keep (used by Story 6.4)
├── inverted.rs         # Keep (used by Story 6.4)
├── cache.rs            # Keep calculate_file_hash function
├── hybrid.rs           # Keep (will be refactored in Story 6.5)
└── ... other files
```

### Code Reuse - CRITICAL

**DO NOT reinvent file hash calculation.** Use existing function:
```rust
// In src-tauri/src/indexer/cache.rs line 89
pub fn calculate_file_hash(file_path: &Path) -> Result<String, CacheError>
```
This uses blake3 for fast parallel hashing (10x faster than SHA256).

**DO NOT reinvent path resolution.** Use existing function:
```rust
// In src-tauri/src/storage/paths.rs line 18
pub fn get_indexes_dir(app_handle: &AppHandle) -> Result<PathBuf, PathError>
```
Update this to support .sqlite files alongside existing .idx files.

### Connection Pool Design

```rust
pub struct SqliteConnectionPool {
    /// Single write connection (WAL allows one writer)
    write_conn: Mutex<Connection>,

    /// Multiple read connections (WAL allows concurrent readers)
    read_conns: Vec<Mutex<Connection>>,

    /// Database path for health checks
    db_path: PathBuf,
}

impl SqliteConnectionPool {
    pub fn new(db_path: &Path, read_pool_size: usize) -> Result<Self, SqliteError> {
        // Open write connection with full access
        let write_conn = Connection::open(db_path)?;
        configure_connection(&write_conn)?;

        // Open read connections with SQLITE_OPEN_READ_ONLY
        let read_conns = (0..read_pool_size)
            .map(|_| {
                let conn = Connection::open_with_flags(
                    db_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                )?;
                configure_connection(&conn)?;
                Ok(Mutex::new(conn))
            })
            .collect::<Result<Vec<_>, SqliteError>>()?;

        Ok(Self { write_conn: Mutex::new(write_conn), read_conns, db_path })
    }

    /// Get exclusive write connection (blocks if in use)
    pub fn get_write_connection(&self) -> MutexGuard<Connection> {
        self.write_conn.lock().unwrap()
    }

    /// Get any available read connection
    pub fn get_read_connection(&self) -> MutexGuard<Connection> {
        // Simple round-robin or first-available
        for conn in &self.read_conns {
            if let Ok(guard) = conn.try_lock() {
                return guard;
            }
        }
        // If all busy, wait for first one
        self.read_conns[0].lock().unwrap()
    }
}
```

### Database Path Strategy

```rust
pub fn get_sqlite_database_path(
    app_handle: &AppHandle,
    file_hash: &str
) -> Result<PathBuf, PathError> {
    let indexes_dir = get_indexes_dir(app_handle)?;
    Ok(indexes_dir.join(format!("{}.sqlite", file_hash)))
}
```

### Testing Requirements

**Unit Tests (90% coverage target for indexer module):**
- Schema creation succeeds and tables exist
- All indexes created with correct columns
- PRAGMA values match expected configuration
- Connection pool provides concurrent read access
- Write connection has exclusive access
- Hash-based lookup creates new DB when none exists
- Hash-based lookup returns existing DB when hash matches
- Hash-based lookup deletes and recreates when hash differs

**Property-Based Tests (proptest):**
- Random file paths produce valid database paths
- Connection pool handles concurrent access without deadlock

### Integration with Existing Code

**Update indexer/mod.rs:**
```rust
// Add new module
pub mod sqlite;

// Re-export public API
pub use sqlite::{SqliteConnectionPool, get_or_create_database};
```

**Do NOT modify yet:**
- `commands/indexation.rs` - Story 6.2 will integrate the pipeline
- `hybrid.rs` - Story 6.5 will refactor or remove
- `tiered.rs`, `interner.rs` - Story 6.5 will remove

### Performance Expectations

- Connection open: <50ms
- Schema creation: <100ms
- PRAGMA application: <10ms
- Database file size: ~100 bytes per entry (vs ~80 bytes for rkyv)

### Project Structure Notes

- All new code in `src-tauri/src/indexer/sqlite/` submodule
- Follows existing module organization pattern (mod.rs with re-exports)
- Uses existing storage/paths.rs for directory resolution
- Uses existing indexer/cache.rs for file hashing

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.1] - Acceptance criteria and schema
- [Source: _bmad-output/project-context.md#Technology-Stack] - Error handling pattern (thiserror)
- [Source: _bmad-output/project-context.md#Rust-Backend-Rules] - Naming conventions, module organization
- [Source: src-tauri/src/indexer/cache.rs:89] - calculate_file_hash function to reuse
- [Source: src-tauri/src/storage/paths.rs:18] - get_indexes_dir function to reuse
- [Source: rusqlite docs] - Connection pool pattern with WAL mode

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- In-memory SQLite databases don't support WAL mode (return "memory" instead) - handled gracefully
- mmap_size PRAGMA may return no rows for in-memory databases - handled with fallback to 0

### Completion Notes List

- ✅ Added rusqlite 0.32 dependency with bundled, backup, functions features
- ✅ Implemented schema.rs with entries and file_info tables, all 7 indexes
- ✅ Implemented connection.rs with PRAGMA optimizations (WAL, 64MB cache, 256MB mmap)
- ✅ Implemented pool.rs with 1 write + N read connections, round-robin selection, graceful shutdown
- ✅ Implemented cache.rs with SHA256-based hash lookup, database validation, cleanup on hash mismatch
- ✅ Created module structure with proper re-exports in indexer/mod.rs
- ✅ All 45 SQLite module tests pass (after removing 4 duplicate hash tests)
- ✅ All indexer module tests pass (no regressions)
- ✅ Build succeeds with new dependency
- ✅ Code review passed - all HIGH and MEDIUM issues fixed

### Change Log

- 2026-01-22: Initial implementation of SQLite schema and connection infrastructure (Story 6.1)
- 2026-01-22: Code review fixes applied:
  - Fixed H1: Removed duplicate `calculate_file_hash_for_sqlite`, now uses shared `calculate_file_hash` from indexer/cache.rs
  - Fixed M1: Added explicit lifetime annotations to MutexGuard returns in pool.rs (lines 157, 169, 198)
  - Fixed M2: Added error type re-exports to indexer/mod.rs (PoolError, SqliteCacheError, SchemaError, ConnectionError, verify_schema, configure_connection)
  - Removed 4 duplicate hash tests (covered by indexer::cache::tests)

### File List

- src-tauri/Cargo.toml (modified - added rusqlite dependency)
- src-tauri/src/indexer/mod.rs (modified - added sqlite module and re-exports)
- src-tauri/src/indexer/sqlite/mod.rs (new)
- src-tauri/src/indexer/sqlite/schema.rs (new)
- src-tauri/src/indexer/sqlite/connection.rs (new)
- src-tauri/src/indexer/sqlite/pool.rs (new)
- src-tauri/src/indexer/sqlite/cache.rs (new)
