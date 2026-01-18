use tauri::{command, AppHandle, Emitter, State};
use crate::api_client::types::{AliasMapping, ApiCredentials, ConnectionInfo, ConnectionStatus, ConnectionTestResult, InterfaceMappingCache};
use crate::api_client::client::test_connection;
use crate::api_client::enrichment::{fetch_aliases_batch, fetch_interface_mappings, fetch_rule_labels_batch};
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

// ============================================================================
// Rule Label Enrichment Commands (Story 3.3)
// ============================================================================

/// Fetch rule labels from OPNsense API for given hashes
#[command]
pub async fn fetch_rule_labels(
    cache_state: State<'_, EnrichmentCacheState>,
    hashes: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    // Load credentials
    let credentials = manager::load_credentials()
        .ok()
        .flatten()
        .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten())
        .ok_or("No API credentials saved. Please configure OPNsense API connection first.")?;

    // Fetch from API (batch with parallelization)
    let labels = fetch_rule_labels_batch(&credentials, hashes)
        .await
        .map_err(|e| format!("Failed to fetch rule labels: {}", e))?;

    // Store in cache
    cache_state.set_rule_labels(labels.clone(), credentials.endpoint_url.clone());

    info!("Rule labels fetched and cached: {} entries", labels.len());
    Ok(labels)
}

/// Get cached rule labels
#[command]
pub fn get_rule_labels(
    cache_state: State<'_, EnrichmentCacheState>,
) -> HashMap<String, String> {
    cache_state.get_all_rule_labels()
        .map(|cache| cache.mappings)
        .unwrap_or_default()
}

/// Get single rule label for specific hash
#[command]
pub fn get_rule_label(
    cache_state: State<'_, EnrichmentCacheState>,
    hash: String,
) -> Option<String> {
    cache_state.get_rule_label(&hash)
}

// ============================================================================
// Alias Resolution Commands (Story 3.4)
// ============================================================================

/// Fetch aliases from OPNsense API for given IPs
#[command]
pub async fn fetch_aliases(
    cache_state: State<'_, EnrichmentCacheState>,
    ips: Vec<String>,
) -> Result<HashMap<String, Vec<AliasMapping>>, String> {
    // Load credentials
    let credentials = manager::load_credentials()
        .ok()
        .flatten()
        .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten())
        .ok_or("No API credentials saved. Please configure OPNsense API connection first.")?;

    // Fetch from API (batch with parallelization)
    let alias_map = fetch_aliases_batch(&credentials, ips)
        .await
        .map_err(|e| format!("Failed to fetch aliases: {}", e))?;

    // Store in cache
    cache_state.set_aliases(alias_map.clone(), credentials.endpoint_url.clone());

    info!("Aliases fetched and cached: {} IPs", alias_map.len());
    Ok(alias_map)
}

/// Get cached aliases
#[command]
pub fn get_aliases(
    cache_state: State<'_, EnrichmentCacheState>,
) -> HashMap<String, Vec<AliasMapping>> {
    cache_state.get_all_aliases()
        .map(|cache| cache.mappings)
        .unwrap_or_default()
}

/// Get aliases for specific IP
#[command]
pub fn get_alias(
    cache_state: State<'_, EnrichmentCacheState>,
    ip: String,
) -> Option<Vec<AliasMapping>> {
    cache_state.get_alias(&ip)
}

// ============================================================================
// Connection Management Commands (Story 3.5)
// ============================================================================

/// Get current API connection status
#[command]
pub fn get_connection_status(
    cache_state: State<'_, EnrichmentCacheState>,
) -> ConnectionInfo {
    cache_state.get_connection_info()
}

/// Manually retry API connection
#[command]
pub async fn retry_api_connection(
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<ConnectionInfo, String> {
    info!("Manual connection retry requested");

    // Load credentials
    let credentials = manager::load_credentials()
        .ok()
        .flatten()
        .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten())
        .ok_or("No API credentials saved. Please configure OPNsense API connection first.")?;

    // Attempt connection test
    match test_connection(&credentials).await {
        Ok(_) => {
            info!("Connection retry succeeded");
            cache_state.set_connection_status(ConnectionStatus::Connected, None);
            Ok(cache_state.get_connection_info())
        }
        Err(e) => {
            warn!("Connection retry failed: {}", e);
            let error_msg = format!("Connection failed: {}", e);
            cache_state.set_connection_status(ConnectionStatus::Disconnected, Some(error_msg.clone()));
            Err(error_msg)
        }
    }
}
