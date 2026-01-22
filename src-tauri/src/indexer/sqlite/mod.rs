//! SQLite-Based Log Indexation Module
//!
//! Story 6.1: Provides SQLite infrastructure for high-performance log storage and querying.
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

pub mod schema;
pub mod connection;
pub mod pool;
pub mod cache;

// Re-export public API
pub use schema::{create_schema, verify_schema, SchemaError};
pub use connection::{configure_connection, ConnectionError};
pub use pool::{SqliteConnectionPool, PoolError};
pub use cache::{get_or_create_database, SqliteCacheError};
