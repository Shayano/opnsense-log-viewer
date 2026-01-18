use tauri::command;
use crate::api_client::types::{ApiCredentials, ConnectionTestResult};
use crate::api_client::client::test_connection;
use crate::credentials::{manager, encrypted_storage};
use log::{info, warn};

/// Save API credentials to OS keychain (with encrypted fallback)
#[command]
pub async fn save_api_credentials(
    endpoint_url: String,
    api_key: String,
    api_secret: String,
) -> Result<(), String> {
    // Validate inputs
    if endpoint_url.is_empty() || api_key.is_empty() || api_secret.is_empty() {
        return Err("All fields are required".to_string());
    }

    // Enforce HTTPS
    if !endpoint_url.starts_with("https://") {
        return Err("HTTPS required for security. Use https:// instead of http://".to_string());
    }

    let credentials = ApiCredentials {
        endpoint_url,
        api_key,
        api_secret,
        profile_name: None,
    };

    // Try OS keychain first
    match manager::save_credentials(&credentials) {
        Ok(_) => {
            info!("Credentials saved to OS keychain");
            Ok(())
        }
        Err(e) => {
            // Fallback to encrypted file storage
            warn!("Keychain unavailable: {}. Using encrypted fallback.", e);
            encrypted_storage::encrypt_and_save(&credentials)
                .map_err(|e| format!("Failed to save credentials: {}", e))?;
            Ok(())
        }
    }
}

/// Load API credentials from OS keychain (with encrypted fallback)
#[command]
pub async fn load_api_credentials() -> Result<Option<ApiCredentials>, String> {
    // Try OS keychain first
    match manager::load_credentials() {
        Ok(Some(creds)) => {
            info!("Credentials loaded from OS keychain");
            Ok(Some(creds))
        }
        Ok(None) | Err(_) => {
            // Fallback to encrypted file storage
            info!("Keychain empty or unavailable. Trying encrypted fallback.");
            encrypted_storage::decrypt_and_load()
                .map_err(|e| format!("Failed to load credentials: {}", e))
        }
    }
}

/// Test connection to OPNsense API
#[command]
pub async fn test_api_connection(
    endpoint_url: String,
    api_key: String,
    api_secret: String,
) -> Result<ConnectionTestResult, String> {
    // Validate inputs
    if endpoint_url.is_empty() || api_key.is_empty() || api_secret.is_empty() {
        return Err("All fields are required".to_string());
    }

    // Enforce HTTPS
    if !endpoint_url.starts_with("https://") {
        return Err("HTTPS required for security. Use https:// instead of http://".to_string());
    }

    let credentials = ApiCredentials {
        endpoint_url: endpoint_url.clone(),
        api_key,
        api_secret,
        profile_name: None,
    };

    test_connection(&credentials)
        .await
        .map_err(|e| {
            // Format user-friendly error messages
            if e.to_string().contains("Authentication failed") {
                "Authentication failed: Invalid API key or secret. Verify credentials in OPNsense.".to_string()
            } else if e.to_string().contains("Connection refused") {
                format!("Connection refused: Unable to reach {}. Check firewall and network settings.", endpoint_url)
            } else if e.to_string().contains("certificate") {
                format!("TLS certificate error: {}. Enable 'Accept Invalid Certificates' in Advanced settings (not recommended).", e)
            } else if e.to_string().contains("timeout") {
                "Request timeout: OPNsense API did not respond within 10 seconds.".to_string()
            } else {
                format!("Connection failed: {}", e)
            }
        })
}
