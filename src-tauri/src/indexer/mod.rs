pub mod inverted;
pub mod bitmap;
pub mod offset_table;
pub mod progress;
pub mod hybrid;
pub mod parallel;
pub mod streaming;    // Story 6.1: Memory-efficient streaming indexation
pub mod interner;     // Story 6.1: String interning for memory efficiency
pub mod tiered;       // Story 6.1: Tiered index architecture (hot/warm tiers)
pub mod progressive;  // Story 6.3: Thread-safe progressive index wrapper
pub mod cache;        // Story 6.4: Persistent index cache for instant reload

// Re-export public API
pub use inverted::InvertedIndex;
pub use bitmap::BitmapIndex;
pub use offset_table::OffsetTable;
pub use progress::IndexProgress;
pub use hybrid::{HybridIndex, IndexMetadata, IndexError};
pub use parallel::build_index_parallel;
pub use streaming::build_index_streaming;
pub use interner::{StringInterner, StringKey, LocalInterner};
pub use tiered::{TieredIndex, TieredConfig, HotIndex, WarmIndex, TieredQueryExecutor};
pub use progressive::ProgressiveIndex;  // Story 6.3: Thread-safe wrapper
pub use cache::{IndexCache, CacheMetadata, IndexCacheEvent, calculate_file_hash};  // Story 6.4
