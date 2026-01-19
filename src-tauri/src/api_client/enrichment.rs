use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::task::JoinSet;
use tokio::sync::Semaphore;
use std::sync::Arc;
use log::{debug, info, warn, error};

use crate::api_client::client::build_api_client;
use crate::api_client::types::{
    AliasMapping, AliasSearchResponse, ApiCredentials, ApiError,
    InterfaceMappingResponse, RuleLabelResponse
};

/// Fetch interface name mappings from OPNsense API
///
/// Calls GET /api/diagnostics/interface/getInterfaceNames
/// Returns HashMap: physical_name → logical_name
///
/// Example OPNsense response:
/// {
///   "vtnet0": "lan",
///   "vtnet1": "wan",
///   "vtnet2": "opt1"
/// }
pub async fn fetch_interface_mappings(
    credentials: &ApiCredentials,
) -> Result<HashMap<String, String>> {
    let client = build_api_client(credentials)?;

    let url = format!(
        "{}/api/diagnostics/interface/getInterfaceNames",
        credentials.endpoint_url
    );

    debug!("Fetching interface mappings from OPNsense API");

    let response = client
        .get(&url)
        .header("X-API-Key", &credentials.api_key)
        .header("X-API-Secret", &credentials.api_secret)
        .send()
        .await
        .context("Failed to fetch interface mappings")?;

    if !response.status().is_success() {
        if response.status() == 401 {
            return Err(ApiError::AuthError.into());
        }
        return Err(ApiError::NetworkError(format!("HTTP {}", response.status())).into());
    }

    let raw_mappings: InterfaceMappingResponse = response
        .json()
        .await
        .context("Failed to parse interface mappings response")?;

    // OPNsense returns lowercase logical names (e.g., "lan", "wan", "opt1")
    // Convert to uppercase for better display (e.g., "LAN", "WAN", "OPT1")
    let normalized_mappings: HashMap<String, String> = raw_mappings
        .into_iter()
        .map(|(physical, logical)| {
            let normalized_logical = normalize_logical_name(&logical);
            (physical, normalized_logical)
        })
        .collect();

    info!("Fetched {} interface mappings", normalized_mappings.len());
    Ok(normalized_mappings)
}

/// Normalize logical interface name to uppercase
///
/// OPNsense convention:
/// - "lan" → "LAN"
/// - "wan" → "WAN"
/// - "opt1" → "OPT1"
/// - "dmz" → "DMZ"
fn normalize_logical_name(name: &str) -> String {
    name.to_uppercase()
}

// ============================================================================
// Rule Label Enrichment (Story 3.3)
// ============================================================================

/// Fetch single rule label from OPNsense API
///
/// Calls POST /api/firewall/filter/searchRule
/// Request body: { "current": 1, "rowCount": 1, "searchPhrase": "<rule_hash>" }
///
/// Returns Some(description) if found, None if not found
async fn fetch_rule_label(
    credentials: &ApiCredentials,
    rule_hash: &str,
) -> Result<Option<String>> {
    let client = build_api_client(credentials)?;

    let url = format!("{}/api/firewall/filter/searchRule", credentials.endpoint_url);

    debug!("Fetching rule label for hash: {}", rule_hash);

    let request_body = serde_json::json!({
        "current": 1,
        "rowCount": 1,
        "searchPhrase": rule_hash
    });

    let response = client
        .post(&url)
        .header("X-API-Key", &credentials.api_key)
        .header("X-API-Secret", &credentials.api_secret)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&request_body)?)
        .send()
        .await
        .context("Failed to fetch rule label")?;

    if !response.status().is_success() {
        if response.status() == 401 {
            return Err(ApiError::AuthError.into());
        }
        return Err(ApiError::NetworkError(format!("HTTP {}", response.status())).into());
    }

    // Parse response - OPNsense returns { "rows": [...], "total": N }
    let response_json: serde_json::Value = response.json().await
        .context("Failed to parse rule label response")?;

    // Extract first rule from rows array
    if let Some(rows) = response_json.get("rows").and_then(|r| r.as_array()) {
        if let Some(first_rule) = rows.first() {
            let rule: RuleLabelResponse = serde_json::from_value(first_rule.clone())
                .context("Failed to parse rule object")?;

            return Ok(rule.description);
        }
    }

    // No rule found
    Ok(None)
}

/// Fetch multiple rule labels in parallel (batch optimization)
///
/// Calls fetch_rule_label() for each hash concurrently (up to 10 parallel)
/// Returns HashMap: hash → description (only includes found rules)
pub async fn fetch_rule_labels_batch(
    credentials: &ApiCredentials,
    rule_hashes: Vec<String>,
) -> Result<HashMap<String, String>> {
    info!("Fetching {} rule labels in batch", rule_hashes.len());

    // Limit concurrent requests to 10 to avoid overwhelming OPNsense API
    const MAX_CONCURRENT_REQUESTS: usize = 10;

    let mut tasks = JoinSet::new();
    let credentials = credentials.clone();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS));

    // Spawn parallel tasks with semaphore limiting concurrency
    for hash in rule_hashes {
        let credentials_clone = credentials.clone();
        let hash_clone = hash.clone();
        let sem = semaphore.clone();

        tasks.spawn(async move {
            // Acquire semaphore permit before making request
            let _permit = match sem.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    error!("Semaphore acquisition failed for hash: {}", hash_clone);
                    return (hash_clone, Err(anyhow::anyhow!("Semaphore acquisition failed")));
                }
            };
            let result = fetch_rule_label(&credentials_clone, &hash_clone).await;
            (hash_clone, result)
        });
    }

    // Collect results
    let mut labels = HashMap::new();
    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok((hash, Ok(Some(description)))) => {
                labels.insert(hash, description);
                success_count += 1;
            }
            Ok((hash, Ok(None))) => {
                debug!("Rule label not found for hash: {}", hash);
            }
            Ok((hash, Err(e))) => {
                warn!("Failed to fetch rule label for {}: {}", hash, e);
                error_count += 1;
            }
            Err(e) => {
                error!("Task join error: {}", e);
                error_count += 1;
            }
        }
    }

    info!("Rule label fetch complete: {} found, {} errors", success_count, error_count);
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_logical_name() {
        assert_eq!(normalize_logical_name("lan"), "LAN");
        assert_eq!(normalize_logical_name("wan"), "WAN");
        assert_eq!(normalize_logical_name("opt1"), "OPT1");
        assert_eq!(normalize_logical_name("dmz"), "DMZ");
        assert_eq!(normalize_logical_name("guest"), "GUEST");
    }

    // Additional tests with mock HTTP client will be added in integration tests
}

// ============================================================================
// Alias Resolution (Story 3.4)
// ============================================================================

/// Fetch aliases for a single IP address from OPNsense API
///
/// Calls POST /api/firewall/alias/searchItem
/// Request body: { "item": "<ip_address>" }
///
/// Returns Vec<AliasMapping> (may be empty if IP not aliased)
async fn fetch_aliases_for_ip(
    credentials: &ApiCredentials,
    ip: &str,
) -> Result<Vec<AliasMapping>> {
    let client = build_api_client(credentials)?;

    let url = format!("{}/api/firewall/alias/searchItem", credentials.endpoint_url);

    log::debug!("Fetching aliases for IP: {}", ip);

    let request_body = serde_json::json!({
        "item": ip
    });

    let response = client
        .post(&url)
        .header("X-API-Key", &credentials.api_key)
        .header("X-API-Secret", &credentials.api_secret)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&request_body)?)
        .send()
        .await
        .context("Failed to fetch aliases")?;

    if !response.status().is_success() {
        if response.status() == 401 {
            return Err(ApiError::AuthError.into());
        }
        return Err(ApiError::NetworkError(format!("HTTP {}", response.status())).into());
    }

    // Parse response
    let response_data: AliasSearchResponse = response.json().await
        .context("Failed to parse alias response")?;

    // Convert rows to AliasMapping
    let aliases: Vec<AliasMapping> = response_data.rows.into_iter().map(|row| {
        let group_members: Vec<String> = row.content
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        AliasMapping {
            alias_name: row.name,
            group_members,
            description: row.description,
            alias_type: row.alias_type,
        }
    }).collect();

    Ok(aliases)
}

/// Fetch aliases for multiple IPs in parallel (batch optimization)
///
/// Calls fetch_aliases_for_ip() for each IP concurrently (up to 10 parallel)
/// Returns HashMap: IP → Vec<AliasMapping> (only includes aliased IPs)
pub async fn fetch_aliases_batch(
    credentials: &ApiCredentials,
    ips: Vec<String>,
) -> Result<HashMap<String, Vec<AliasMapping>>> {
    info!("Fetching aliases for {} IPs in batch", ips.len());

    const MAX_CONCURRENT_REQUESTS: usize = 10;
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    let mut tasks = JoinSet::new();
    let credentials = credentials.clone();

    // Spawn parallel tasks with semaphore limiting
    for ip in ips {
        let credentials_clone = credentials.clone();
        let ip_clone = ip.clone();
        let semaphore_clone = Arc::clone(&semaphore);

        tasks.spawn(async move {
            let _permit = match semaphore_clone.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    error!("Semaphore acquisition failed for IP: {}", ip_clone);
                    return (ip_clone, Err(anyhow::anyhow!("Semaphore acquisition failed")));
                }
            };
            let result = fetch_aliases_for_ip(&credentials_clone, &ip_clone).await;
            (ip_clone, result)
        });
    }

    // Collect results
    let mut alias_map = HashMap::new();
    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok((ip, Ok(aliases))) if !aliases.is_empty() => {
                alias_map.insert(ip, aliases);
                success_count += 1;
            }
            Ok((ip, Ok(_))) => {
                // IP not aliased (empty response)
                debug!("No aliases found for IP: {}", ip);
            }
            Ok((ip, Err(e))) => {
                warn!("Failed to fetch aliases for {}: {}", ip, e);
                error_count += 1;
            }
            Err(e) => {
                error!("Task join error: {}", e);
                error_count += 1;
            }
        }
    }

    info!("Alias fetch complete: {} aliased, {} errors", success_count, error_count);
    Ok(alias_map)
}

// ============================================================================
// Enrichment Export Helpers (Story 4.1)
// ============================================================================

/// Fetch OPNsense version from firmware API
///
/// Returns None if API unavailable or version not found
/// This is optional metadata for enrichment exports
pub async fn fetch_opnsense_version(
    credentials: &ApiCredentials,
) -> Result<Option<String>> {
    let client = build_api_client(credentials)?;

    let url = format!("{}/api/core/firmware/status", credentials.endpoint_url);

    debug!("Fetching OPNsense version for export metadata");

    let response = client
        .get(&url)
        .header("X-API-Key", &credentials.api_key)
        .header("X-API-Secret", &credentials.api_secret)
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                // Extract version from response
                let version = json.get("product_version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                if let Some(ref v) = version {
                    debug!("OPNsense version: {}", v);
                }

                Ok(version)
            } else {
                debug!("Failed to parse firmware status response");
                Ok(None)
            }
        }
        Ok(resp) => {
            debug!("Failed to parse firmware status response or non-success status: {}", resp.status());
            Ok(None)
        }
        Err(e) => {
            // API unavailable or error - not critical for export
            debug!("Failed to fetch OPNsense version (non-critical): {}", e);
            Ok(None)
        }
    }
}

/// Calculate configuration hash for change detection
///
/// Hash is deterministic - same data produces same hash
/// Uses SHA-256 to generate hex string (64 characters)
pub fn calculate_config_hash(
    interfaces: &HashMap<String, String>,
    rule_labels: &HashMap<String, String>,
    aliases: &HashMap<String, Vec<String>>,
) -> String {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();

    // Serialize data to JSON (without metadata to avoid timestamp changing hash)
    // Note: HashMap iteration order is not deterministic in Rust, but serde_json
    // sorts keys alphabetically, making this deterministic
    let hash_input = format!(
        "{}{}{}",
        serde_json::to_string(interfaces).unwrap_or_default(),
        serde_json::to_string(rule_labels).unwrap_or_default(),
        serde_json::to_string(aliases).unwrap_or_default(),
    );

    hasher.update(hash_input.as_bytes());
    let result = hasher.finalize();

    // Return hex-encoded hash
    format!("{:x}", result)
}

#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn test_config_hash_deterministic() {
        let mut interfaces = HashMap::new();
        interfaces.insert("vtnet0".to_string(), "LAN".to_string());
        interfaces.insert("vtnet1".to_string(), "WAN".to_string());

        let mut rule_labels = HashMap::new();
        rule_labels.insert("abc123".to_string(), "Block RFC1918".to_string());

        let mut aliases = HashMap::new();
        aliases.insert("Servers".to_string(), vec!["192.168.1.100".to_string()]);

        let hash1 = calculate_config_hash(&interfaces, &rule_labels, &aliases);
        let hash2 = calculate_config_hash(&interfaces, &rule_labels, &aliases);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
        assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 hex characters");
    }

    #[test]
    fn test_config_hash_changes_with_data() {
        let mut interfaces1 = HashMap::new();
        interfaces1.insert("vtnet0".to_string(), "LAN".to_string());

        let mut interfaces2 = HashMap::new();
        interfaces2.insert("vtnet0".to_string(), "LAN".to_string());
        interfaces2.insert("vtnet1".to_string(), "WAN".to_string());

        let rule_labels = HashMap::new();
        let aliases = HashMap::new();

        let hash1 = calculate_config_hash(&interfaces1, &rule_labels, &aliases);
        let hash2 = calculate_config_hash(&interfaces2, &rule_labels, &aliases);

        assert_ne!(hash1, hash2, "Hash should change when data changes");
    }

    #[test]
    fn test_config_hash_with_empty_data() {
        let interfaces = HashMap::new();
        let rule_labels = HashMap::new();
        let aliases = HashMap::new();

        let hash = calculate_config_hash(&interfaces, &rule_labels, &aliases);

        assert_eq!(hash.len(), 64, "Hash should work with empty data");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hex");
    }
}

// ============================================================================
// Enrichment Import Helpers (Story 4.2)
// ============================================================================

/// Staleness threshold for enrichment data (in days)
/// Data older than this threshold triggers a warning during import
pub const ENRICHMENT_STALENESS_THRESHOLD_DAYS: i64 = 7;

/// Calculate age of enrichment data in days
///
/// Returns the number of days since the export timestamp
/// Used to determine if enrichment data is stale (>ENRICHMENT_STALENESS_THRESHOLD_DAYS days)
pub fn calculate_enrichment_age(export_timestamp: &chrono::DateTime<chrono::Utc>) -> Result<i64> {
    let now = chrono::Utc::now();
    let age = now.signed_duration_since(*export_timestamp);

    // Return age in days (fractional days rounded down)
    Ok(age.num_days())
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_calculate_enrichment_age_recent() {
        let recent = Utc::now() - Duration::hours(12);
        let age = calculate_enrichment_age(&recent).unwrap();
        assert_eq!(age, 0, "Recent timestamp should be 0 days old");
    }

    #[test]
    fn test_calculate_enrichment_age_old() {
        let old = Utc::now() - Duration::days(10);
        let age = calculate_enrichment_age(&old).unwrap();
        assert_eq!(age, 10, "10-day old timestamp should return 10 days");
    }

    #[test]
    fn test_calculate_enrichment_age_exactly_7_days() {
        let seven_days = Utc::now() - Duration::days(7);
        let age = calculate_enrichment_age(&seven_days).unwrap();
        assert_eq!(age, 7);
    }

    #[test]
    fn test_calculate_enrichment_age_future() {
        // Edge case: future timestamp (should handle gracefully)
        let future = Utc::now() + Duration::days(5);
        let age = calculate_enrichment_age(&future).unwrap();
        assert!(age < 0, "Future timestamp should return negative age");
    }
}
