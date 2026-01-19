use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    /// CSV format (RFC 4180)
    Csv,

    /// JSON format with pretty-print
    Json,
}

/// Export scope - filtered results or full dataset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportScope {
    /// Export only filtered results
    Filtered,

    /// Export entire unfiltered dataset
    FullDataset,
}

/// Source file information for export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFileInfo {
    /// Original log file path
    pub path: PathBuf,

    /// SHA-256 hash of source file
    pub sha256: String,
}

/// Filter information for export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterInfo {
    /// Filter field (e.g., "action", "sourceIp")
    pub field: String,

    /// Filter operator (e.g., "equals", "contains")
    pub operator: String,

    /// Filter value (e.g., "block", "192.168.1.0/24")
    pub value: String,
}

/// Enrichment status for export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentInfo {
    /// Whether enrichment is active
    pub active: bool,

    /// Enrichment source: "Live API" or "Backup (exported YYYY-MM-DD)"
    pub source: String,

    /// Timestamp of enrichment data (for backup)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<DateTime<Utc>>,
}

/// Export checksum for integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportChecksum {
    /// Checksum algorithm (always "SHA-256")
    pub algorithm: String,

    /// Hex-encoded checksum
    pub hash: String,
}

/// Export verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportVerification {
    /// Number of entries successfully written
    pub entries_written: usize,

    /// Whether export completed successfully
    pub export_complete: bool,
}

/// Complete export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetadata {
    /// Application name and version
    pub exported_by: String,

    /// Export timestamp
    pub export_date: DateTime<Utc>,

    /// Source file information
    pub source_file: SourceFileInfo,

    /// Filters that were applied
    pub filters_applied: Vec<FilterInfo>,

    /// Number of entries in export
    pub total_entries: usize,

    /// Total entries in source file
    pub total_in_source: usize,

    /// Export scope - filtered or full dataset
    pub export_scope: ExportScope,

    /// Enrichment status (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment_status: Option<EnrichmentInfo>,

    /// Export checksum for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_checksum: Option<ExportChecksum>,

    /// Verification status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<ExportVerification>,
}

/// Export progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    /// Current number of entries exported
    pub current: usize,

    /// Total entries to export
    pub total: usize,

    /// Export status message
    pub status: String,

    /// Export speed (rows per second)
    pub rows_per_second: f64,

    /// Elapsed time in seconds
    pub elapsed_seconds: f64,
}

/// Export result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// Path to exported file
    pub file_path: PathBuf,

    /// Number of entries written
    pub entries_written: usize,

    /// Total time taken (seconds)
    pub duration_seconds: f64,

    /// File size in bytes
    pub file_size_bytes: u64,

    /// SHA-256 checksum of exported file
    pub checksum: String,

    /// Whether verification passed (entry count matches)
    pub verification_passed: bool,
}

/// Export request from frontend
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    /// Export format (CSV or JSON)
    pub format: ExportFormat,

    /// Export metadata
    pub metadata: ExportMetadata,

    /// Save file path
    pub save_path: PathBuf,
}

/// Log entry for export (simplified, enriched)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportLogEntry {
    /// Timestamp (ISO 8601)
    pub timestamp: String,

    /// Interface (enriched: "LAN" or raw: "vtnet0")
    pub interface: String,

    /// Source IP
    pub source_ip: String,

    /// Source port
    pub source_port: u16,

    /// Destination IP
    pub destination_ip: String,

    /// Destination port
    pub destination_port: u16,

    /// Protocol (TCP, UDP, ICMP, etc.)
    pub protocol: String,

    /// Action (pass, block, reject)
    pub action: String,

    /// Rule label (enriched: "Block RFC1918" or raw: "abc123def")
    pub rule_label: String,
}

/// Export estimate for warning dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEstimate {
    /// Estimated number of entries to export
    pub estimated_entries: usize,

    /// Estimated file size in MB
    pub estimated_file_size_mb: f64,

    /// Estimated duration in seconds
    pub estimated_duration_seconds: f64,
}

/// Insufficient disk space error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsufficientDiskSpaceError {
    /// Required disk space in MB
    pub required_mb: f64,

    /// Available disk space in MB
    pub available_mb: f64,
}

/// Verification result for post-export verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    /// Whether checksum is valid
    pub valid: bool,

    /// Expected checksum from file metadata
    pub expected_hash: String,

    /// Actual checksum calculated from file
    pub actual_hash: String,

    /// File size in bytes
    pub file_size_bytes: u64,

    /// Number of entries in file (if parseable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_count: Option<usize>,
}
