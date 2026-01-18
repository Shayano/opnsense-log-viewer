use serde::{Deserialize, Serialize};

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
