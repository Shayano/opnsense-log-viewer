/// Storage module for index persistence
///
/// This module handles saving and loading index files to/from disk with compression
/// and integrity verification.

pub mod integrity;
pub mod persistence;
pub mod paths;

// Re-export public APIs
pub use integrity::{calculate_checksum, calculate_file_hash, calculate_file_hash_quick, verify_checksum, IntegrityError};
pub use persistence::{load_index, save_index, PersistenceError};
pub use paths::{get_index_path, get_indexes_dir, PathError};
