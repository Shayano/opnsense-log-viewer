//! SQLite-Based Log Indexation Module
//!
//! Story 6.1: Provides SQLite infrastructure for high-performance log storage and querying.
//! Story 6.2: Adds parallel parsing pipeline with Rayon and crossbeam channels.
//!
//! ## Architecture
//!
//! This module replaces the rkyv/mmap hybrid approach that crashes at 72M entries (32-bit pointer limit).
//! SQLite provides:
//! - **No 32-bit pointer limit** - Handles unlimited entries
//! - **Disk-based storage** - Bounded memory usage regardless of file size
//! - **WAL mode** - Concurrent reads during write, crash recovery
//! - **SQL queries** - Replaces custom bitmap/inverted index logic
//!
//! ## Module Structure
//!
//! - `schema`: Database schema definition (tables, indexes)
//! - `connection`: PRAGMA configuration for optimal performance
//! - `pool`: Connection pool with read/write management
//! - `cache`: Hash-based database lookup and management
//! - `parsed_entry`: Entry format for SQLite insertion
//! - `progress`: Pipeline progress tracking with atomic counters
//! - `error`: Pipeline error types
//! - `parallel_parser`: Rayon-based parallel log parsing
//! - `batch_writer`: Transaction-batched SQLite writer
//! - `pipeline`: Pipeline orchestrator

// Story 6.1: Core SQLite infrastructure
pub mod schema;
pub mod connection;
pub mod pool;
pub mod cache;

// Story 6.2: Parallel parsing pipeline
pub mod parsed_entry;
pub mod progress;
pub mod error;
pub mod parallel_parser;
pub mod batch_writer;
pub mod pipeline;

// Re-export public API - Story 6.1
pub use schema::{create_schema, verify_schema, SchemaError};
pub use connection::{configure_connection, register_regexp_function, ConnectionError};
pub use pool::{SqliteConnectionPool, PoolError};
pub use cache::{get_or_create_database, SqliteCacheError};

// Re-export public API - Story 6.2
pub use parsed_entry::ParsedEntry;
pub use progress::{PipelineProgress, ProgressSnapshot};
pub use error::{PipelineError, PipelineResult};
pub use parallel_parser::{parse_file_parallel, parse_file_streaming};
pub use batch_writer::BatchWriter;
pub use pipeline::{build_sqlite_index, build_sqlite_index_with_capacity, IndexStats};
