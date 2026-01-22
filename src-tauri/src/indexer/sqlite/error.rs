//! Pipeline Error Types
//!
//! Story 6.2: Defines error types for the parallel parsing pipeline.
//!
//! Uses `thiserror` for library-style errors as per project conventions.

use thiserror::Error;

use super::connection::ConnectionError;
use super::pool::PoolError;
use super::schema::SchemaError;

/// Errors that can occur during pipeline operations
#[derive(Error, Debug)]
pub enum PipelineError {
    /// IO error (file operations)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Database error (SQLite operations)
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Connection pool error
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),

    /// Schema error
    #[error("Schema error: {0}")]
    Schema(#[from] SchemaError),

    /// Connection configuration error
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    /// Parse error (non-fatal, aggregated)
    #[error("Parse errors: {count} entries failed to parse")]
    ParseErrors { count: u64 },

    /// Writer thread panicked
    #[error("Writer thread panicked")]
    WriterPanic,

    /// Channel closed unexpectedly
    #[error("Channel closed unexpectedly")]
    ChannelClosed,

    /// Pipeline cancelled by user
    #[error("Pipeline cancelled")]
    Cancelled,

    /// Invalid format specified
    #[error("Invalid log format: {0}")]
    InvalidFormat(String),

    /// File hash mismatch (corruption or file changed)
    #[error("File hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Timeout waiting for operation
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },
}

impl PipelineError {
    /// Check if this error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            PipelineError::Io(_)
            | PipelineError::Database(_)
            | PipelineError::Pool(_)
            | PipelineError::Timeout { .. }
        )
    }

    /// Check if this error indicates user cancellation
    pub fn is_cancelled(&self) -> bool {
        matches!(self, PipelineError::Cancelled)
    }
}

/// Result type alias for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let pipeline_err: PipelineError = io_err.into();

        assert!(matches!(pipeline_err, PipelineError::Io(_)));
        assert!(pipeline_err.is_recoverable());
    }

    #[test]
    fn test_error_display() {
        let err = PipelineError::ParseErrors { count: 42 };
        let display = format!("{}", err);

        assert!(display.contains("42"));
        assert!(display.contains("parse"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(PipelineError::Timeout { seconds: 30 }.is_recoverable());
        assert!(!PipelineError::WriterPanic.is_recoverable());
        assert!(!PipelineError::Cancelled.is_recoverable());
    }

    #[test]
    fn test_is_cancelled() {
        assert!(PipelineError::Cancelled.is_cancelled());
        assert!(!PipelineError::WriterPanic.is_cancelled());
    }

    #[test]
    fn test_hash_mismatch() {
        let err = PipelineError::HashMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        let display = format!("{}", err);

        assert!(display.contains("abc123"));
        assert!(display.contains("def456"));
    }
}
