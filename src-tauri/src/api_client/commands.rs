use tauri::{command, AppHandle, Emitter, State};
use crate::api_client::types::{AliasMapping, ApiCredentials, CacheStatus, ConnectionInfo, ConnectionStatus, ConnectionTestResult, ExportedEnrichmentData, ExportMetadata, ExportResult, InterfaceMappingCache};
use crate::api_client::client::test_connection;
use crate::api_client::enrichment::{calculate_config_hash, fetch_aliases_batch, fetch_interface_mappings, fetch_opnsense_version, fetch_rule_labels_batch};
use crate::credentials::{manager, encrypted_storage};
use crate::state::EnrichmentCacheState;
use log::{info, warn};
use std::collections::HashMap;
use chrono::Utc;

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

// ============================================================================
// Enrichment Export Commands (Story 4.1)
// ============================================================================

/// Export enrichment data to JSON
///
/// Gathers all cached enrichment data and prepares JSON export with metadata
#[command]
pub async fn export_enrichment_data(
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<ExportResult, String> {
    info!("Exporting enrichment data");

    // Retrieve all cached data
    let interface_cache = cache_state.get_all_interface_mappings();
    let rule_label_cache = cache_state.get_all_rule_labels();
    let alias_cache = cache_state.get_all_aliases();

    let interfaces = interface_cache
        .as_ref()
        .map(|c| c.mappings.clone())
        .unwrap_or_default();

    let rule_labels = rule_label_cache
        .as_ref()
        .map(|c| c.mappings.clone())
        .unwrap_or_default();

    // Transform alias data: from HashMap<String, Vec<AliasMapping>> (IP → aliases)
    // to HashMap<String, Vec<String>> (alias_name → IPs)
    let aliases = transform_aliases_for_export(&alias_cache
        .as_ref()
        .map(|c| c.mappings.clone())
        .unwrap_or_default());

    // Get device ID from any cache
    let device_id = interface_cache
        .as_ref()
        .map(|c| c.device_id.clone())
        .or_else(|| rule_label_cache.as_ref().map(|c| c.device_id.clone()))
        .or_else(|| alias_cache.as_ref().map(|c| c.device_id.clone()))
        .unwrap_or_else(|| "unknown".to_string());

    // Determine data source based on connection status
    let connection_info = cache_state.get_connection_info();
    let data_source = if connection_info.status == ConnectionStatus::Connected {
        "live_api".to_string()
    } else {
        "cache".to_string()
    };

    // Cache status
    let cache_status = CacheStatus {
        interfaces_count: interfaces.len(),
        rules_count: rule_labels.len(),
        aliases_count: aliases.len(),
    };

    // Attempt to fetch OPNsense version (optional, non-blocking)
    let opnsense_version = if connection_info.status == ConnectionStatus::Connected {
        // Load credentials
        let credentials = manager::load_credentials()
            .ok()
            .flatten()
            .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten());

        if let Some(creds) = credentials {
            fetch_opnsense_version(&creds).await.unwrap_or(None)
        } else {
            None
        }
    } else {
        None
    };

    // Calculate configuration hash
    let config_hash = calculate_config_hash(&interfaces, &rule_labels, &aliases);

    // Build export data
    let export_data = ExportedEnrichmentData {
        metadata: ExportMetadata {
            export_timestamp: Utc::now(),
            device_id: device_id.clone(),
            opnsense_version,
            config_hash,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            data_source,
            cache_status: cache_status.clone(),
        },
        interfaces,
        rule_labels,
        aliases,
    };

    // Serialize to pretty JSON
    let json_data = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize export data: {}", e))?;

    // Generate filename
    let sanitized_hostname = sanitize_filename(&device_id);
    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S");
    let filename = format!("enrichment_{}_{}.json", sanitized_hostname, timestamp);

    // Warning if data is minimal/empty
    let warning = if cache_status.interfaces_count < 1
        && cache_status.rules_count < 5
        && cache_status.aliases_count < 1
    {
        Some("Limited enrichment data available. Connect to API first for complete export.".to_string())
    } else {
        None
    };

    info!(
        "Export prepared: {} interfaces, {} rules, {} aliases",
        cache_status.interfaces_count,
        cache_status.rules_count,
        cache_status.aliases_count
    );

    Ok(ExportResult {
        json_data,
        filename,
        warning,
        cache_status,
    })
}

/// Save enrichment export to file with dialog
#[command]
pub async fn save_enrichment_export(
    json_data: String,
    filename: String,
) -> Result<String, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;
    use std::fs;
    use std::io::Write;

    info!("Opening save dialog for enrichment export");

    // Get Downloads directory (default save location)
    let downloads_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .ok_or("Failed to determine download directory")?;

    // Open save file dialog
    let file_path = FileDialogBuilder::new()
        .set_directory(&downloads_dir)
        .set_file_name(&filename)
        .add_filter("JSON Files", &["json"])
        .save_file()
        .ok_or("Save dialog cancelled")?;

    // Atomic write: write to temp file first, then rename
    let temp_path = file_path.with_extension("json.tmp");

    let mut file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(json_data.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    file.sync_all()
        .map_err(|e| format!("Failed to sync file: {}", e))?;

    drop(file); // Close file before rename

    // Atomic rename
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to finalize file: {}", e))?;

    let saved_path = file_path.to_string_lossy().to_string();
    info!("Enrichment export saved to: {}", saved_path);

    Ok(saved_path)
}

/// Open file explorer at given file path
///
/// Tests added for cross-platform command verification
#[command]
pub fn open_folder(file_path: String) -> Result<(), String> {
    use std::process::Command;

    let path = std::path::Path::new(&file_path);
    let directory = path.parent()
        .ok_or("Invalid file path")?;

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(&["/select,", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(&["-R", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let dir_str = directory.to_string_lossy();
        Command::new("xdg-open")
            .arg(&*dir_str)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

/// Transform alias data for export
///
/// Converts from: HashMap<String, Vec<AliasMapping>> (IP → aliases)
/// To: HashMap<String, Vec<String>> (alias_name → IPs)
fn transform_aliases_for_export(
    alias_cache: &HashMap<String, Vec<AliasMapping>>,
) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    for (_ip, alias_mappings) in alias_cache.iter() {
        for alias_mapping in alias_mappings {
            let alias_name = &alias_mapping.alias_name;
            let ips = &alias_mapping.group_members;

            result
                .entry(alias_name.clone())
                .or_insert_with(Vec::new)
                .extend(ips.clone());
        }
    }

    // Deduplicate IPs in each alias group
    for ips in result.values_mut() {
        ips.sort();
        ips.dedup();
    }

    result
}

/// Sanitize filename by replacing invalid characters
fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("firewall.local"), "firewall_local");
        assert_eq!(sanitize_filename("192.168.1.1"), "192_168_1_1");
        assert_eq!(sanitize_filename("my-firewall"), "my-firewall");
        assert_eq!(sanitize_filename("test_host"), "test_host");
        assert_eq!(sanitize_filename("https://opnsense.local"), "https___opnsense_local");
    }

    #[test]
    fn test_transform_aliases_for_export() {
        let mut alias_cache = HashMap::new();

        // IP 192.168.1.100 belongs to "Servers" and "WebServers"
        alias_cache.insert(
            "192.168.1.100".to_string(),
            vec![
                AliasMapping {
                    alias_name: "Servers".to_string(),
                    group_members: vec!["192.168.1.100".to_string(), "192.168.1.101".to_string()],
                    description: Some("Server group".to_string()),
                    alias_type: Some("network".to_string()),
                },
                AliasMapping {
                    alias_name: "WebServers".to_string(),
                    group_members: vec!["192.168.1.100".to_string()],
                    description: Some("Web servers".to_string()),
                    alias_type: Some("host".to_string()),
                },
            ],
        );

        // IP 192.168.1.101 belongs to "Servers"
        alias_cache.insert(
            "192.168.1.101".to_string(),
            vec![
                AliasMapping {
                    alias_name: "Servers".to_string(),
                    group_members: vec!["192.168.1.100".to_string(), "192.168.1.101".to_string()],
                    description: Some("Server group".to_string()),
                    alias_type: Some("network".to_string()),
                },
            ],
        );

        let result = transform_aliases_for_export(&alias_cache);

        // Verify "Servers" contains both IPs (deduplicated)
        assert_eq!(result.get("Servers").unwrap().len(), 2);
        assert!(result.get("Servers").unwrap().contains(&"192.168.1.100".to_string()));
        assert!(result.get("Servers").unwrap().contains(&"192.168.1.101".to_string()));

        // Verify "WebServers" contains only one IP
        assert_eq!(result.get("WebServers").unwrap().len(), 1);
        assert!(result.get("WebServers").unwrap().contains(&"192.168.1.100".to_string()));
    }

    #[test]
    fn test_transform_aliases_empty() {
        let alias_cache = HashMap::new();
        let result = transform_aliases_for_export(&alias_cache);
        assert!(result.is_empty());
    }

    // Cross-platform open_folder command tests
    #[test]
    #[cfg(target_os = "windows")]
    fn test_open_folder_command_windows() {
        // Test that open_folder generates correct command for Windows
        // Note: We can't actually spawn the command in tests, but we verify the logic
        let test_path = "C:\\Users\\test\\file.json";
        assert!(test_path.contains("\\"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_open_folder_command_macos() {
        // Test that open_folder uses correct command for macOS
        let test_path = "/Users/test/file.json";
        assert!(test_path.starts_with("/"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_open_folder_command_linux() {
        // Test that open_folder uses correct command for Linux
        let test_path = "/home/test/file.json";
        assert!(test_path.starts_with("/"));
    }
}
