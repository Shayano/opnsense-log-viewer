use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// API credentials for OPNsense connection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCredentials {
    pub endpoint_url: String,
    pub api_key: String,
    pub api_secret: String,
    #[serde(default)]
    pub profile_name: Option<String>, // For future multi-profile support
    /// Accept invalid TLS certificates (self-signed certificates)
    /// WARNING: This disables certificate validation. Use only for trusted local networks.
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

/// Connection status for UI indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    /// API is reachable and working
    Connected,
    /// API is unreachable or credentials invalid
    Disconnected,
    /// API partially working (some calls timing out)
    Degraded,
    /// Using imported backup enrichment data (Story 4.2)
    #[serde(rename = "backup_enrichment")]
    BackupEnrichment,
}

/// Connection status with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub status: ConnectionStatus,
    pub last_error: Option<String>,
    pub last_checked: DateTime<Utc>,
}

/// Result from test connection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub success: bool,
    pub interface_count: usize,
    pub opnsense_version: Option<String>,
    pub error_message: Option<String>,
}

/// API error types
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication failed: Invalid API key or secret")]
    AuthError,

    #[error("TLS certificate error: {0}")]
    TlsError(String),

    #[error("Request timeout after {0} seconds")]
    TimeoutError(u64),

    #[error("HTTP endpoint not allowed (HTTPS required)")]
    HttpNotAllowed,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

// ============================================================================
// Interface Mapping Types (Story 3.2)
// ============================================================================

/// Single interface mapping from physical to logical name
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceMapping {
    pub physical_name: String,
    pub logical_name: String,
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default)]
    pub ip_address: Option<String>,
}

/// Cached interface mappings with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceMappingCache {
    pub mappings: HashMap<String, String>, // physical → logical
    pub last_updated: DateTime<Utc>,
    pub device_id: String, // OPNsense endpoint URL or hostname
}

/// OPNsense API response format for /api/diagnostics/interface/getInterfaceNames
/// Response format: { "vtnet0": "lan", "vtnet1": "wan", "vtnet2": "opt1" }
/// Note: OPNsense returns lowercase logical names, we'll titlecase them
pub type InterfaceMappingResponse = HashMap<String, String>;

// ============================================================================
// Rule Label Enrichment Types (Story 3.3)
// ============================================================================

/// Single rule label mapping from hash to description
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleLabelMapping {
    pub rule_hash: String,
    pub description: String,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_enabled: bool,
}

/// Cached rule labels with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleLabelCache {
    pub mappings: HashMap<String, String>, // hash → description
    pub last_updated: DateTime<Utc>,
    pub device_id: String, // OPNsense endpoint URL
}

/// OPNsense API response format for /api/firewall/filter/searchRule
/// Response includes rule object with "descr" field for description
#[derive(Debug, Deserialize)]
pub struct RuleLabelResponse {
    pub uuid: Option<String>,
    #[serde(rename = "descr")]
    pub description: Option<String>,
    pub enabled: Option<String>, // "1" or "0"
}

// ============================================================================
// Alias Resolution Types (Story 3.4)
// ============================================================================

/// Single alias mapping for an IP
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasMapping {
    pub alias_name: String,
    pub group_members: Vec<String>,  // IPs in this alias group
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub alias_type: Option<String>,  // network, host, port, url, etc.
}

/// Cached alias mappings with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasCache {
    pub mappings: HashMap<String, Vec<AliasMapping>>, // IP → [Alias1, Alias2]
    pub last_updated: DateTime<Utc>,
    pub device_id: String, // OPNsense endpoint URL
}

/// OPNsense API response format for /api/firewall/alias/searchItem
/// Response includes rows array with alias objects
#[derive(Debug, Deserialize)]
pub struct AliasSearchResponse {
    pub rows: Vec<AliasRow>,
}

#[derive(Debug, Deserialize)]
pub struct AliasRow {
    #[serde(rename = "uuid")]
    pub alias_uuid: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub alias_type: Option<String>,
    pub content: String,  // Comma-separated IPs or values
    #[serde(rename = "descr")]
    pub description: Option<String>,
}

// ============================================================================
// Enrichment Export Types (Story 4.1)
// ============================================================================

/// Cache status counts for export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pub interfaces_count: usize,
    pub rules_count: usize,
    pub aliases_count: usize,
}

/// Metadata for enrichment export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetadata {
    /// Export creation timestamp (ISO 8601)
    pub export_timestamp: DateTime<Utc>,

    /// OPNsense endpoint URL (without credentials)
    pub device_id: String,

    /// OPNsense version (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opnsense_version: Option<String>,

    /// Configuration hash for change detection (SHA-256)
    pub config_hash: String,

    /// Application version that created the export
    pub app_version: String,

    /// Source of enrichment data ("live_api" or "cache")
    pub data_source: String,

    /// Cache status - number of entries per type
    pub cache_status: CacheStatus,
}

/// Complete enrichment export data
///
/// SECURITY: This structure MUST NOT contain any API credentials.
/// Only enrichment mappings are exported for offline use.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedEnrichmentData {
    /// Export metadata
    pub metadata: ExportMetadata,

    /// Interface mappings: physical_name → logical_name
    /// Example: {"vtnet0": "LAN", "vtnet1": "WAN"}
    pub interfaces: HashMap<String, String>,

    /// Rule labels: rule_hash → description
    /// Example: {"abc123": "Block RFC1918 Networks"}
    pub rule_labels: HashMap<String, String>,

    /// Aliases: alias_name → [IP addresses]
    /// Example: {"Servers_Group": ["192.168.1.100", "192.168.1.101"]}
    pub aliases: HashMap<String, Vec<String>>,
}

/// Export command result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// Exported data (serialized JSON)
    pub json_data: String,

    /// Suggested filename
    pub filename: String,

    /// Warning if enrichment data is minimal/empty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,

    /// Cache status for UI display
    pub cache_status: CacheStatus,
}

// ============================================================================
// Enrichment Import Types (Story 4.2)
// ============================================================================

/// Validation result for enrichment import
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportValidation {
    /// Whether validation passed
    pub is_valid: bool,

    /// Specific error message if validation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Missing fields (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_fields: Option<Vec<String>>,

    /// Age of enrichment in days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_days: Option<i64>,

    /// Whether enrichment is stale (>7 days)
    pub is_stale: bool,

    /// Export metadata from file (if valid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExportMetadata>,
}

/// Import command result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Number of interfaces imported
    pub interfaces_imported: usize,

    /// Number of rule labels imported
    pub rules_imported: usize,

    /// Number of aliases imported
    pub aliases_imported: usize,

    /// Export timestamp from file
    pub export_timestamp: DateTime<Utc>,

    /// Device source identifier
    pub device_id: String,

    /// Age in days
    pub age_days: i64,
}

// ============================================================================
// Staleness Indicator Types (Story 4.3)
// ============================================================================

/// Staleness threshold constant (from Story 4.2)
pub const ENRICHMENT_STALENESS_THRESHOLD_DAYS: i64 = 7;

/// Severity level for backup enrichment age
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StalenessSeverity {
    /// Fresh enrichment (<24 hours old)
    Fresh,

    /// Moderately stale (1-7 days old)
    Moderate,

    /// Highly stale (>7 days old) - verify accuracy
    High,
}

impl StalenessSeverity {
    /// Calculate severity based on age in days
    pub fn from_age_days(age_days: i64) -> Self {
        if age_days < 1 {
            Self::Fresh
        } else if age_days <= ENRICHMENT_STALENESS_THRESHOLD_DAYS {
            Self::Moderate
        } else {
            Self::High
        }
    }

    /// Get background color class for Tailwind CSS
    pub fn background_color(&self) -> &'static str {
        match self {
            Self::Fresh => "bg-yellow-100 dark:bg-yellow-900/20",
            Self::Moderate => "bg-amber-100 dark:bg-amber-900/30",
            Self::High => "bg-orange-100 dark:bg-orange-900/40",
        }
    }

    /// Get text color class for Tailwind CSS
    pub fn text_color(&self) -> &'static str {
        match self {
            Self::Fresh => "text-yellow-800 dark:text-yellow-200",
            Self::Moderate => "text-amber-800 dark:text-amber-200",
            Self::High => "text-orange-800 dark:text-orange-200",
        }
    }
}

/// Result of API reconnection attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionResult {
    /// Whether connection succeeded
    pub connected: bool,

    /// OPNsense version if connected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opnsense_version: Option<String>,

    /// Error message if connection failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Number of interfaces fetched (if connected)
    pub interfaces_count: usize,

    /// Number of rules fetched (if connected)
    pub rules_count: usize,

    /// Number of aliases fetched (if connected)
    pub aliases_count: usize,
}

/// Event payload for API reconnection detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiReconnectedEvent {
    /// Current API status
    pub api_status: String,

    /// Whether backup enrichment is active
    pub backup_active: bool,

    /// Timestamp of reconnection
    pub reconnected_at: DateTime<Utc>,
}
