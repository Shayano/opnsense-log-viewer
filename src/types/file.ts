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
