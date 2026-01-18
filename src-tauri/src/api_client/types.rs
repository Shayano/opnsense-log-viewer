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
