pub mod inverted;
pub mod bitmap;
pub mod offset_table;
pub mod progress;
pub mod hybrid;
pub mod parallel;
pub mod streaming;  // Story 6.1: Memory-efficient streaming indexation
pub mod interner;   // Story 6.1: String interning for memory efficiency

// Re-export public API
pub use inverted::InvertedIndex;
pub use bitmap::BitmapIndex;
pub use offset_table::OffsetTable;
pub use progress::IndexProgress;
pub use hybrid::{HybridIndex, IndexMetadata, IndexError};
pub use parallel::build_index_parallel;
pub use streaming::build_index_streaming;
pub use interner::{StringInterner, StringKey, LocalInterner};
