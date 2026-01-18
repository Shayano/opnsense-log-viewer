use tauri::{command, AppHandle, Emitter, State};
use crate::api_client::types::{ApiCredentials, ConnectionTestResult, InterfaceMappingCache};
use crate::api_client::client::test_connection;
use crate::api_client::enrichment::fetch_interface_mappings;
use crate::credentials::{manager, encrypted_storage};
use crate::state::EnrichmentCacheState;
use log::{info, warn};
use std::collections::HashMap;

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
///
/// MODIFIED: Auto-fetch interface mappings on successful connection
#[command]
pub async fn test_api_connection(
    endpoint_url: String,
    api_key: String,
    api_secret: String,
    app_handle: AppHandle,
    cache_state: State<'_, EnrichmentCacheState>,
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

    let result = test_connection(&credentials)
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
        })?;

    // Auto-fetch interface mappings on successful connection
    if result.success {
        let credentials_clone = credentials.clone();
        let cache_state_clone = (*cache_state).clone();
        let app_handle_clone = app_handle.clone();

        // Spawn background task to fetch mappings (don't block connection test)
        tokio::spawn(async move {
            match fetch_interface_mappings(&credentials_clone).await {
                Ok(mappings) => {
                    // Cache mappings
                    cache_state_clone.set_interface_mappings(
                        mappings.clone(),
                        credentials_clone.endpoint_url.clone(),
                    );

                    // Emit event to frontend
                    let _ = app_handle_clone.emit("interface-mappings-updated", mappings);

                    info!("Interface mappings auto-fetched on connection success");
                }
                Err(e) => {
                    warn!("Failed to auto-fetch interface mappings: {}", e);
                }
            }
        });
    }

    Ok(result)
}

// ============================================================================
// Interface Mapping Commands (Story 3.2)
// ============================================================================

/// Fetch interface mappings from OPNsense API and cache
#[command]
pub async fn fetch_interface_mappings_cmd(
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<HashMap<String, String>, String> {
    // Load credentials
    let credentials = manager::load_credentials()
        .ok()
        .flatten()
        .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten())
        .ok_or("No API credentials saved. Please configure OPNsense API connection first.")?;

    // Fetch from API
    let mappings = fetch_interface_mappings(&credentials)
        .await
        .map_err(|e| format!("Failed to fetch interface mappings: {}", e))?;

    // Store in cache
    cache_state.set_interface_mappings(mappings.clone(), credentials.endpoint_url.clone());

    info!("Interface mappings fetched and cached");
    Ok(mappings)
}

/// Get cached interface mappings
#[command]
pub fn get_interface_mappings_cmd(
    cache_state: State<'_, EnrichmentCacheState>,
) -> Option<InterfaceMappingCache> {
    cache_state.get_all_interface_mappings()
}

/// Get logical name for a specific physical interface
#[command]
pub fn get_logical_interface_name(
    cache_state: State<'_, EnrichmentCacheState>,
    physical_name: String,
) -> Option<String> {
    cache_state.get_interface_mapping(&physical_name)
}
