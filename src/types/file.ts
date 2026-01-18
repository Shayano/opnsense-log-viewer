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
 * Result from load_index_file command
 */
export type LoadIndexResult =
  | { type: 'Success'; metadata: SourceFileMetadata }
  | { type: 'NotFound' }
  | { type: 'HashMismatch'; storedHash: string; actualHash: string };
