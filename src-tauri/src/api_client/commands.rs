use tauri::{command, AppHandle, Emitter, State};
use crate::api_client::types::{AliasMapping, ApiCredentials, CacheStatus, ConnectionInfo, ConnectionStatus, ConnectionTestResult, ExportedEnrichmentData, ExportMetadata, ExportResult, ImportResult, ImportValidation, InterfaceMappingCache};
use crate::api_client::client::test_connection;
use crate::api_client::enrichment::{calculate_config_hash, calculate_enrichment_age, fetch_aliases_batch, fetch_all_aliases, fetch_all_rule_labels, fetch_interface_mappings, fetch_opnsense_version, fetch_rule_labels_batch, ENRICHMENT_STALENESS_THRESHOLD_DAYS};
use crate::credentials::{manager, encrypted_storage};
use crate::state::EnrichmentCacheState;
use log::{info, warn};
use std::collections::HashMap;
use chrono::Utc;
use tokio::time::{timeout, Duration};

/// Save API credentials to OS keychain (with encrypted fallback)
#[command]
pub async fn save_api_credentials(
    endpoint_url: String,
    api_key: String,
    api_secret: String,
    accept_invalid_certs: Option<bool>,
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
        accept_invalid_certs: accept_invalid_certs.unwrap_or(false),
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
    accept_invalid_certs: Option<bool>,
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
        accept_invalid_certs: accept_invalid_certs.unwrap_or(false),
    };

    let result = test_connection(&credentials)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            // Format user-friendly error messages with specific error detection
            if error_str.contains("Authentication failed") {
                "Authentication failed: Invalid API key or secret. Verify credentials in OPNsense.".to_string()
            } else if error_str.contains("API returned HTML") || error_str.contains("HTML instead of JSON") {
                // Extract the helpful message from InvalidResponse
                if error_str.contains("API returned HTML") {
                    // Return the detailed message from InvalidResponse
                    error_str
                } else {
                    format!("API returned HTML instead of JSON. This usually means:\n\
                            1. The endpoint URL is incorrect or the port is wrong\n\
                            2. Authentication failed and you were redirected to a login page\n\
                            3. The API endpoint doesn't exist on this OPNsense instance\n\
                            \n\
                            Verify that:\n\
                            - The endpoint URL is correct (e.g., https://192.168.1.1 or https://192.168.1.1:443)\n\
                            - The port number is correct (usually the same as the web interface, typically 443 for HTTPS)\n\
                            - The API key and secret are valid\n\
                            - The OPNsense API is enabled in System > Settings > API")
                }
            } else if error_str.contains("TLS certificate") 
                || error_str.contains("certificate verify failed")
                || error_str.contains("invalid peer certificate")
                || error_str.contains("unable to get local issuer certificate")
                || error_str.contains("self signed certificate")
                || error_str.contains("certificate signed by unknown authority") {
                format!("TLS certificate validation failed: The server uses a self-signed or invalid certificate. Enable 'Accept Invalid Certificates' option above and try again. Error details: {}", error_str)
            } else if error_str.contains("Connection refused") {
                format!("Connection refused: Unable to reach {}. Check firewall and network settings.", endpoint_url)
            } else if error_str.contains("timeout") {
                "Request timeout: OPNsense API did not respond within 10 seconds.".to_string()
            } else {
                format!("Connection failed: {}", error_str)
            }
        })?;

    // Auto-fetch interfaces only on successful connection (no log file required)
    // NOTE: Rule labels and aliases are fetched on-demand only to prevent memory leaks
    if result.success {
        let credentials_clone = credentials.clone();
        let cache_state_clone = (*cache_state).clone();
        let app_handle_clone = app_handle.clone();

        // Spawn lightweight task for interface mapping only with timeout
        tokio::spawn(async move {
            match timeout(Duration::from_secs(30), fetch_interface_mappings(&credentials_clone)).await {
                Ok(Ok(mappings)) => {
                    cache_state_clone.set_interface_mappings(
                        mappings.clone(),
                        credentials_clone.endpoint_url.clone(),
                    );
                    let _ = app_handle_clone.emit("interface-mappings-updated", mappings);
                    info!("Interface mappings auto-fetched on connection success");
                }
                Ok(Err(e)) => warn!("Failed to auto-fetch interface mappings: {}", e),
                Err(_) => warn!("Interface mapping fetch timed out after 30 seconds"),
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
    info!("fetch_rule_labels called with {} hashes", hashes.len());
    // Load credentials
    let credentials = manager::load_credentials()
        .ok()
        .flatten()
        .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten())
        .ok_or("No API credentials saved. Please configure OPNsense API connection first.")?;

    info!("Rule labels: loaded credentials for {}", credentials.endpoint_url);

    // Debug: check current cache state
    let current_rule_labels = cache_state.get_all_rule_labels();
    info!("Current rule labels cache: exists={}, size={}",
          current_rule_labels.is_some(),
          current_rule_labels.as_ref().map(|c| c.mappings.len()).unwrap_or(0));

    // Check if we have any rule labels cached
    let existing_labels = cache_state.get_all_rule_labels();
    let has_cached_data = existing_labels.as_ref().map(|c| !c.mappings.is_empty()).unwrap_or(false);

    info!("Rule labels enrichment: has_cached_data={}, existing_cache_size={}",
          has_cached_data,
          existing_labels.as_ref().map(|c| c.mappings.len()).unwrap_or(0));

    // If no cached data, fetch ALL rules to populate cache (exhaustive enrichment needed)
    if !has_cached_data {
        info!("No rule labels cached, fetching ALL rules for exhaustive enrichment");
        match fetch_all_rule_labels(&credentials).await {
            Ok(all_labels) => {
                let labels_count = all_labels.len();
                cache_state.set_rule_labels(all_labels, credentials.endpoint_url.clone());
                info!("ALL rule labels cached: {} entries for exhaustive enrichment", labels_count);
            }
            Err(e) => {
                warn!("Failed to fetch all rule labels: {}", e);
                // Continue with specific hash lookup even if initial fetch fails
            }
        }
    } else {
        info!("Rule labels already cached, skipping exhaustive fetch");
    }

    // Now fetch specific hashes (they might be in cache now, or we fetch them individually)
    let labels = fetch_rule_labels_batch(&credentials, hashes)
        .await
        .map_err(|e| format!("Failed to fetch rule labels: {}", e))?;

    // Store in cache (clone to avoid moving)
    let result_labels = labels.clone();
    if let Some(existing) = existing_labels {
        let mut merged_labels = existing.mappings;
        merged_labels.extend(labels);
        cache_state.set_rule_labels(merged_labels, credentials.endpoint_url.clone());
    } else {
        cache_state.set_rule_labels(labels, credentials.endpoint_url.clone());
    }

    info!("Rule labels fetched and cached: {} entries", result_labels.len());
    Ok(result_labels)
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
    info!("fetch_aliases called with {} IPs", ips.len());
    // Load credentials
    let credentials = manager::load_credentials()
        .ok()
        .flatten()
        .or_else(|| encrypted_storage::decrypt_and_load().ok().flatten())
        .ok_or("No API credentials saved. Please configure OPNsense API connection first.")?;

    info!("Aliases: loaded credentials for {}", credentials.endpoint_url);

    // Debug: check current cache state
    let current_aliases = cache_state.get_all_aliases();
    info!("Current aliases cache: exists={}, size={}",
          current_aliases.is_some(),
          current_aliases.as_ref().map(|c| c.mappings.len()).unwrap_or(0));

    // Check if we have any aliases cached
    let existing_aliases = cache_state.get_all_aliases();
    let has_cached_data = existing_aliases.as_ref().map(|c| !c.mappings.is_empty()).unwrap_or(false);

    info!("Aliases enrichment: has_cached_data={}, existing_cache_size={}",
          has_cached_data,
          existing_aliases.as_ref().map(|c| c.mappings.len()).unwrap_or(0));

    // If no cached data, fetch ALL aliases to populate cache (exhaustive enrichment needed)
    if !has_cached_data {
        info!("No aliases cached, fetching ALL aliases for exhaustive enrichment");
        match fetch_all_aliases(&credentials).await {
            Ok(all_aliases) => {
                let aliases_count = all_aliases.len();
                cache_state.set_aliases(all_aliases, credentials.endpoint_url.clone());
                info!("ALL aliases cached: {} IPs covered for exhaustive enrichment", aliases_count);
            }
            Err(e) => {
                warn!("Failed to fetch all aliases: {}", e);
                // Continue with specific IP lookup even if initial fetch fails
            }
        }
    } else {
        info!("Aliases already cached, skipping exhaustive fetch");
    }

    // Now fetch specific IPs (they might be covered by cached aliases now)
    let alias_map = fetch_aliases_batch(&credentials, ips)
        .await
        .map_err(|e| format!("Failed to fetch aliases: {}", e))?;

    // Store in cache (clone to avoid moving)
    let result_aliases = alias_map.clone();
    if let Some(existing) = existing_aliases {
        let mut merged_aliases = existing.mappings;
        merged_aliases.extend(alias_map);
        cache_state.set_aliases(merged_aliases, credentials.endpoint_url.clone());
    } else {
        cache_state.set_aliases(alias_map, credentials.endpoint_url.clone());
    }

    info!("Aliases fetched and cached: {} IPs", result_aliases.len());
    Ok(result_aliases)
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
    app: AppHandle,
    json_data: String,
    filename: String,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    use std::fs;
    use std::io::Write;

    info!("Opening save dialog for enrichment export");

    // Get Downloads directory (default save location)
    let downloads_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .ok_or("Failed to determine download directory")?;

    // Open save file dialog
    let file_path = app.dialog()
        .file()
        .set_directory(&downloads_dir)
        .set_file_name(&filename)
        .add_filter("JSON Files", &["json"])
        .blocking_save_file()
        .ok_or("Save dialog cancelled")?;

    // Convert FilePath to PathBuf
    let file_path_buf = file_path.into_path()
        .map_err(|e| format!("Failed to convert file path: {}", e))?;

    // Atomic write: write to temp file first, then rename
    let temp_path = file_path_buf.with_extension("json.tmp");

    let mut file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(json_data.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    file.sync_all()
        .map_err(|e| format!("Failed to sync file: {}", e))?;

    drop(file); // Close file before rename

    // Atomic rename
    fs::rename(&temp_path, &file_path_buf)
        .map_err(|e| format!("Failed to finalize file: {}", e))?;

    let saved_path = file_path_buf.to_string_lossy().to_string();
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
    let _directory = path.parent()
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
        let dir_str = _directory.to_string_lossy();
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

// ============================================================================
// Enrichment Import Commands (Story 4.2)
// ============================================================================

/// Validate enrichment JSON file before import
///
/// Checks:
/// - JSON syntax validity
/// - Required fields present (metadata, interfaces, rule_labels, aliases)
/// - Metadata fields present (export_timestamp, device_id, config_hash)
/// - Calculates enrichment age (days since export)
/// - Determines if stale (>7 days)
#[command]
pub fn validate_enrichment_import(file_path: String) -> Result<ImportValidation, String> {
    info!("Validating enrichment import: {}", file_path);

    // Read file contents
    let file_contents = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse JSON
    let export_data: ExportedEnrichmentData = match serde_json::from_str(&file_contents) {
        Ok(data) => data,
        Err(e) => {
            return Ok(ImportValidation {
                is_valid: false,
                error_message: Some(format!("Invalid JSON format: {}", e)),
                missing_fields: None,
                age_days: None,
                is_stale: false,
                metadata: None,
            });
        }
    };

    // Validate required metadata fields
    let mut missing_fields = Vec::new();

    // Validate export_timestamp is present (DateTime<Utc> is always valid if parsed, but check it's not default/zero)
    if export_data.metadata.export_timestamp.timestamp() == 0 {
        missing_fields.push("metadata.exportTimestamp".to_string());
    }
    if export_data.metadata.device_id.is_empty() {
        missing_fields.push("metadata.deviceId".to_string());
    }
    if export_data.metadata.config_hash.is_empty() {
        missing_fields.push("metadata.configHash".to_string());
    }

    if !missing_fields.is_empty() {
        return Ok(ImportValidation {
            is_valid: false,
            error_message: Some("Missing required metadata fields".to_string()),
            missing_fields: Some(missing_fields),
            age_days: None,
            is_stale: false,
            metadata: None,
        });
    }

    // Calculate enrichment age
    let age_days = calculate_enrichment_age(&export_data.metadata.export_timestamp)
        .map_err(|e| format!("Failed to calculate enrichment age: {}", e))?;

    let is_stale = age_days > ENRICHMENT_STALENESS_THRESHOLD_DAYS;

    info!(
        "Validation passed: {} interfaces, {} rules, {} aliases, {} days old",
        export_data.interfaces.len(),
        export_data.rule_labels.len(),
        export_data.aliases.len(),
        age_days
    );

    Ok(ImportValidation {
        is_valid: true,
        error_message: None,
        missing_fields: None,
        age_days: Some(age_days),
        is_stale,
        metadata: Some(export_data.metadata),
    })
}

/// Import enrichment data from JSON file
///
/// Steps:
/// 1. Validates file first (fail fast if invalid)
/// 2. Parses ExportedEnrichmentData
/// 3. Transforms alias data format (alias_name → IPs to IP → aliases)
/// 4. Updates enrichment cache with imported data
/// 5. Returns ImportResult with counts
///
/// On error: Preserves current state (no partial updates)
#[command]
pub async fn import_enrichment_data(
    file_path: String,
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<ImportResult, String> {
    info!("Importing enrichment data from: {}", file_path);

    // Validate first (fail fast if invalid)
    let validation = validate_enrichment_import(file_path.clone())?;
    if !validation.is_valid {
        return Err(validation.error_message.unwrap_or_else(|| "Validation failed".to_string()));
    }

    // Read and parse file
    let file_contents = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let export_data: ExportedEnrichmentData = serde_json::from_str(&file_contents)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Transform aliases: HashMap<alias_name, Vec<IP>> → HashMap<IP, Vec<AliasMapping>>
    let aliases_for_cache = transform_aliases_for_import(&export_data.aliases);

    // Update cache state with imported data
    cache_state.set_interface_mappings(export_data.interfaces.clone(), export_data.metadata.device_id.clone());
    cache_state.set_rule_labels(export_data.rule_labels.clone(), export_data.metadata.device_id.clone());
    cache_state.set_aliases(aliases_for_cache, export_data.metadata.device_id.clone());

    // Update connection status to disconnected (backup enrichment mode)
    // Note: Frontend will handle backup enrichment UI state
    cache_state.set_connection_status(ConnectionStatus::Disconnected, Some("Using backup enrichment".to_string()));

    let age_days = validation.age_days.unwrap_or(0);

    info!(
        "Import complete: {} interfaces, {} rules, {} aliases",
        export_data.interfaces.len(),
        export_data.rule_labels.len(),
        export_data.aliases.len()
    );

    Ok(ImportResult {
        interfaces_imported: export_data.interfaces.len(),
        rules_imported: export_data.rule_labels.len(),
        aliases_imported: export_data.aliases.len(),
        export_timestamp: export_data.metadata.export_timestamp,
        device_id: export_data.metadata.device_id,
        age_days,
    })
}

/// Transform aliases from export format to cache format
///
/// Export format: HashMap<alias_name, Vec<IP>>
/// Cache format: HashMap<IP, Vec<AliasMapping>>
///
/// Example:
/// Input: {"Servers": ["192.168.1.100", "192.168.1.101"], "DMZ": ["10.0.1.5"]}
/// Output: {
///   "192.168.1.100": [AliasMapping { alias_name: "Servers", group_members: [...] }],
///   "192.168.1.101": [AliasMapping { alias_name: "Servers", group_members: [...] }],
///   "10.0.1.5": [AliasMapping { alias_name: "DMZ", group_members: [...] }]
/// }
fn transform_aliases_for_import(
    aliases_export: &HashMap<String, Vec<String>>
) -> HashMap<String, Vec<AliasMapping>> {
    let mut aliases_cache: HashMap<String, Vec<AliasMapping>> = HashMap::new();

    for (alias_name, ips) in aliases_export {
        for ip in ips {
            let mapping = AliasMapping {
                alias_name: alias_name.clone(),
                group_members: ips.clone(),
                description: None,
                alias_type: None,
            };

            aliases_cache
                .entry(ip.clone())
                .or_insert_with(Vec::new)
                .push(mapping);
        }
    }

    aliases_cache
}

/// Open file picker dialog for enrichment import
///
/// Returns:
/// - Some(file_path) if user selects a file
/// - None if user cancels dialog
#[command]
pub async fn open_enrichment_file_picker(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    info!("Opening file picker for enrichment import");

    // Get Downloads directory (default location)
    let downloads_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir())
        .ok_or("Failed to determine download directory")?;

    // Open file picker dialog
    let file_path = app.dialog()
        .file()
        .set_directory(&downloads_dir)
        .add_filter("Enrichment Files", &["json"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            let path_buf = path.into_path()
                .map_err(|e| format!("Failed to convert file path: {}", e))?;
            let path_str: String = path_buf.to_string_lossy().to_string();
            Ok(Some(path_str))
        },
        None => Ok(None), // User cancelled
    }
}

// ============================================================================
// Staleness Indicator Commands (Story 4.3)
// ============================================================================

/// Attempt to reconnect to OPNsense API and fetch fresh enrichment
///
/// Reads stored credentials, attempts connection, fetches enrichment data,
/// and updates cache. Clears backup enrichment state on success.
#[command]
pub async fn reconnect_api(
    cache_state: State<'_, EnrichmentCacheState>,
    app_handle: AppHandle,
) -> Result<crate::api_client::types::ConnectionResult, String> {
    info!("Attempting to reconnect to OPNsense API");

    // Get API credentials from cache state
    let credentials = cache_state.get_credentials()
        .ok_or_else(|| "No API credentials configured. Please set up API connection first.".to_string())?;

    // Test connection
    match test_connection(&credentials).await {
        Ok(_) => {
            info!("API reconnection successful");

            // Fetch interfaces, all rule labels, and all aliases at connection (no log file required)
            let interfaces = fetch_interface_mappings(&credentials).await
                .map_err(|e| format!("Failed to fetch interfaces: {}", e))?;

            cache_state.set_interface_mappings(interfaces.clone(), credentials.endpoint_url.clone());

            let rules_count = match fetch_all_rule_labels(&credentials).await {
                Ok(labels) => {
                    cache_state.set_rule_labels(labels.clone(), credentials.endpoint_url.clone());
                    labels.len()
                }
                Err(e) => {
                    warn!("fetch_all_rule_labels on reconnect failed: {}", e);
                    cache_state.get_all_rule_labels().map(|c| c.mappings.len()).unwrap_or(0)
                }
            };

            let aliases_count = match fetch_all_aliases(&credentials).await {
                Ok(alias_map) => {
                    cache_state.set_aliases(alias_map.clone(), credentials.endpoint_url.clone());
                    alias_map.len()
                }
                Err(e) => {
                    warn!("fetch_all_aliases on reconnect failed: {}", e);
                    cache_state.get_all_aliases().map(|c| c.mappings.len()).unwrap_or(0)
                }
            };

            // Update connection status to Connected
            cache_state.set_connection_status(ConnectionStatus::Connected, None);

            // Clear backup enrichment state
            cache_state.clear_backup_metadata();

            // Emit reconnection event for frontend
            let event = crate::api_client::types::ApiReconnectedEvent {
                api_status: "connected".to_string(),
                backup_active: false,
                reconnected_at: Utc::now(),
            };
            app_handle.emit("api-reconnected", event)
                .map_err(|e| format!("Failed to emit reconnection event: {}", e))?;

            // Fetch OPNsense version (optional metadata)
            let opnsense_version = fetch_opnsense_version(&credentials)
                .await
                .ok()
                .flatten();

            info!("Reconnection complete: {} interfaces, {} rules, {} aliases",
                  interfaces.len(), rules_count, aliases_count);

            Ok(crate::api_client::types::ConnectionResult {
                connected: true,
                opnsense_version,
                error_message: None,
                interfaces_count: interfaces.len(),
                rules_count,
                aliases_count,
            })
        }
        Err(e) => {
            warn!("API reconnection failed: {}", e);

            Ok(crate::api_client::types::ConnectionResult {
                connected: false,
                opnsense_version: None,
                error_message: Some(format!("Connection failed: {}", e)),
                interfaces_count: 0,
                rules_count: 0,
                aliases_count: 0,
            })
        }
    }
}

/// Clear backup enrichment and revert to raw data display
///
/// Clears all enrichment caches (interfaces, rules, aliases),
/// sets connection status to Disconnected, and resets staleness state.
#[command]
pub async fn clear_backup_enrichment(
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<(), String> {
    info!("Clearing backup enrichment data");

    // Clear all enrichment caches
    cache_state.clear_interface_mappings();
    cache_state.clear_rule_labels();
    cache_state.clear_aliases();

    // Set connection status to Disconnected
    cache_state.set_connection_status(ConnectionStatus::Disconnected, None);

    // Clear backup metadata (also resets staleness_indicator_dismissed)
    cache_state.clear_backup_metadata();

    info!("Backup enrichment cleared - using raw data");

    Ok(())
}

/// Set staleness indicator dismissed state (session-scoped)
#[command]
pub fn set_staleness_indicator_dismissed(
    dismissed: bool,
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<(), String> {
    cache_state.set_staleness_dismissed(dismissed);
    info!("Staleness indicator dismissed: {}", dismissed);
    Ok(())
}

/// Get staleness indicator dismissed state
#[command]
pub fn get_staleness_indicator_dismissed(
    cache_state: State<'_, EnrichmentCacheState>,
) -> Result<bool, String> {
    Ok(cache_state.get_staleness_dismissed())
}

#[cfg(test)]
mod import_tests {
    use super::*;

    #[test]
    fn test_transform_aliases_for_import() {
        let mut aliases_export = HashMap::new();
        aliases_export.insert(
            "Servers_Group".to_string(),
            vec!["192.168.1.100".to_string(), "192.168.1.101".to_string()]
        );
        aliases_export.insert(
            "DMZ_Hosts".to_string(),
            vec!["10.0.1.5".to_string()]
        );

        let aliases_cache = transform_aliases_for_import(&aliases_export);

        // Verify IP → aliases mapping
        assert!(aliases_cache.contains_key("192.168.1.100"));
        assert!(aliases_cache.contains_key("192.168.1.101"));
        assert!(aliases_cache.contains_key("10.0.1.5"));

        // Verify aliases for specific IP
        let ip_100_aliases = &aliases_cache["192.168.1.100"];
        assert_eq!(ip_100_aliases.len(), 1);
        assert_eq!(ip_100_aliases[0].alias_name, "Servers_Group");
        assert_eq!(ip_100_aliases[0].group_members, vec!["192.168.1.100", "192.168.1.101"]);
    }

    #[test]
    fn test_transform_aliases_for_import_empty() {
        let aliases_export = HashMap::new();
        let aliases_cache = transform_aliases_for_import(&aliases_export);
        assert!(aliases_cache.is_empty());
    }

    #[test]
    fn test_transform_aliases_for_import_single_ip() {
        let mut aliases_export = HashMap::new();
        aliases_export.insert(
            "WebServer".to_string(),
            vec!["192.168.1.50".to_string()]
        );

        let aliases_cache = transform_aliases_for_import(&aliases_export);

        assert_eq!(aliases_cache.len(), 1);
        assert!(aliases_cache.contains_key("192.168.1.50"));
        let aliases = &aliases_cache["192.168.1.50"];
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].alias_name, "WebServer");
    }

    #[test]
    fn test_transform_aliases_for_import_multiple_aliases_same_ip() {
        // Edge case: Same IP appears in multiple aliases
        let mut aliases_export = HashMap::new();
        aliases_export.insert(
            "Servers".to_string(),
            vec!["192.168.1.100".to_string()]
        );
        aliases_export.insert(
            "WebServers".to_string(),
            vec!["192.168.1.100".to_string()]
        );

        let aliases_cache = transform_aliases_for_import(&aliases_export);

        assert!(aliases_cache.contains_key("192.168.1.100"));
        let aliases = &aliases_cache["192.168.1.100"];
        assert_eq!(aliases.len(), 2, "IP should have 2 alias mappings");

        let alias_names: Vec<String> = aliases.iter().map(|a| a.alias_name.clone()).collect();
        assert!(alias_names.contains(&"Servers".to_string()));
        assert!(alias_names.contains(&"WebServers".to_string()));
    }
}
