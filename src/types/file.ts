/**
 * Information about a selected file
 */
export interface FileInfo {
  /** Absolute path to the file */
  path: string;
  /** File size in bytes */
  size: number;
  /** Detected log format */
  format: 'RFC3164' | 'RFC5424' | 'CSV' | 'UNKNOWN';
  /** Timestamp when file was selected */
  selectedAt: Date;
}

/**
 * Metadata about the indexed file (from Rust backend)
 */
export interface IndexMetadata {
  /** SHA-256 hash of the source file */
  sourceFileHash: string;
  /** Number of log entries indexed */
  entryCount: number;
  /** Detected log format */
  format: 'RFC3164' | 'RFC5424' | 'CSV' | 'UNKNOWN';
  /** Size of the index file in bytes */
  indexSizeBytes: number;
  /** ISO 8601 timestamp when index was created */
  createdAt: string;
}

/**
 * File metadata (size) from Rust backend
 */
export interface FileMetadata {
  /** File size in bytes */
  size: number;
}

/**
 * Information about a saved index (from list_all_indexes command)
 */
export interface IndexInfo {
  /** SHA-256 hash of the source file */
  hash: string;
  /** Path to the source log file */
  sourceFilePath: string;
  /** Whether the source file still exists */
  sourceFileExists: boolean;
  /** ISO 8601 timestamp when index was created */
  createdAt: string;
  /** Size of the source file in bytes */
  fileSize: number;
  /** Number of log entries in the index */
  entryCount: number;
  /** Size of the index file in bytes */
  indexFileSize: number;
}

/**
 * Source file metadata from persisted index
 */
export interface SourceFileMetadata {
  /** File path */
  filePath: string;
  /** File size in bytes */
  fileSize: number;
  /** Number of log entries */
  entryCount: number;
  /** Detected log format */
  logFormat: 'RFC3164' | 'RFC5424' | 'CSV' | 'UNKNOWN';
}

/**
 * Progress data during indexation (from Rust backend)
 * Extended with batch tracking fields from Story 6.3 progressive indexation
 * Story 6.3 AC4: Extended with entriesPerSecond and elapsedSeconds for SQLite streaming
 */
export interface IndexProgress {
  /** Percentage complete (0-100) */
  percentage: number;
  /** Bytes processed so far */
  bytesProcessed: number;
  /** Total bytes to process */
  totalBytes: number;
  /** Current indexation speed in GB/min */
  speedGbps: number;
  /** Estimated seconds remaining */
  etaSeconds: number;
  /** Number of entries indexed so far */
  entriesIndexed: number;
  /** Estimated total entries (refined as indexation progresses) */
  totalEntriesEstimated: number;
  /** Number of batches completed */
  batchesCompleted: number;
  /** Total number of batches */
  totalBatches: number;
  /** True when first batch completes - filtering can begin */
  partialFilterAvailable: boolean;
  /** Story 6.3 AC4: Entries processed per second */
  entriesPerSecond: number;
  /** Story 6.3 AC4: Elapsed time in seconds */
  elapsedSeconds: number;
}

/**
 * Metadata returned after SQLite index building (Story 6.3)
 */
export interface SqliteIndexMetadata {
  /** Total entries successfully indexed */
  entryCount: number;
  /** Total time elapsed in milliseconds */
  elapsedMs: number;
  /** Processing speed in entries per second */
  entriesPerSecond: number;
  /** Bytes processed from source file */
  bytesProcessed: number;
  /** Bytes per second throughput */
  bytesPerSecond: number;
  /** File hash for cache identification */
  fileHash: string;
  /** Detected log format */
  format: string;
}

/**
 * Cache event payload from backend (Story 6.4)
 * Emitted on "index-cache-hit" and "index-cache-miss" events
 */
export interface IndexCacheEvent {
  /** Path to the file being indexed */
  filePath: string;
  /** Whether index was loaded from cache */
  cacheHit: boolean;
  /** Age of cached index in seconds (null if cache miss) */
  cacheAgeSeconds: number | null;
  /** Reason for cache miss (null if cache hit) */
  reason: string | null;
}

/**
 * Note: load_index_file command uses Tauri Result pattern:
 * - Success: Returns IndexMetadata directly
 * - NotFound: Throws error with message containing "No saved index"
 * - HashMismatch: Throws error with message containing "modified since indexing"
 * - Corruption: Throws error with message containing "corrupted" or "Checksum"
 *
 * Handle via try/catch and parse error message to determine case.
 */
