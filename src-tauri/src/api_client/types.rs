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
    Disconnected,
    Connecting,
    Connected,
    Error,
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
