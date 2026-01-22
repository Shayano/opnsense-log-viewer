pub mod inverted;
pub mod bitmap;
pub mod offset_table;
pub mod progress;
pub mod hybrid;
pub mod parallel;
pub mod sqlite;       // Story 6.1 (rewrite): SQLite-based high-performance log indexation

// Re-export public API
pub use inverted::InvertedIndex;
pub use bitmap::BitmapIndex;
pub use offset_table::OffsetTable;
pub use progress::IndexProgress;
pub use hybrid::{HybridIndex, IndexMetadata, IndexError};
pub use parallel::build_index_parallel;

// Story 6.1 (rewrite): SQLite-based indexation
pub use sqlite::{
    SqliteConnectionPool, create_schema, get_or_create_database,
    PoolError, SqliteCacheError, SchemaError, ConnectionError,
    verify_schema, configure_connection,
};
