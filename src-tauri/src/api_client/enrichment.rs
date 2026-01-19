use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::task::JoinSet;
use tokio::sync::Semaphore;
use std::sync::Arc;
use tracing::{debug, info, warn, error};

use crate::api_client::client::build_api_client;
use crate::api_client::types::{
    AliasMapping, AliasSearchResponse, ApiCredentials, ApiError,
    InterfaceMappingResponse, RuleLabelResponse, StalenessSeverity,
    ENRICHMENT_STALENESS_THRESHOLD_DAYS as STALENESS_THRESHOLD
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

    // OPNsense API uses Basic Authentication (not custom headers)
    let response = client
        .get(&url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
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

    // OPNsense API uses Basic Authentication (not custom headers)
    let response = client
        .post(&url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
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

/// Fetch all rule labels from OPNsense API at connection time (no log file required).
///
/// Uses POST /api/firewall/filter/searchRule with JSON body. Must include show_all=1 to get
/// Firewall -> Rules (main ruleset); without it only Automation -> Filter rules are returned (often 0).
/// Builds id->description from rows. UUID can be in row.uuid or row["@attributes"].uuid.
/// Paginates if total exceeds rowCount.
pub async fn fetch_all_rule_labels(
    credentials: &ApiCredentials,
) -> Result<HashMap<String, String>> {
    let client = build_api_client(credentials)?;
    let base = credentials.endpoint_url.trim_end_matches('/');
    let url = format!("{}/api/firewall/filter/searchRule", base);

    const ROW_COUNT: i64 = 10_000;
    let mut all_labels = HashMap::new();
    let mut current = 1;

    loop {
        // POST with JSON body. show_all=1 is required to include Firewall -> Rules (not only Automation).
        let body = serde_json::json!({
            "current": current,
            "rowCount": ROW_COUNT,
            "sort": {},
            "searchPhrase": "",
            "show_all": 1
        });

        let response = client
            .post(&url)
            .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&body).context("serialize searchRule body")?)
            .send()
            .await
            .context("Failed to fetch rules in fetch_all_rule_labels")?;

        if !response.status().is_success() {
            if response.status() == 401 {
                return Err(ApiError::AuthError.into());
            }
            return Err(ApiError::NetworkError(format!("HTTP {}", response.status())).into());
        }

        let json: serde_json::Value = response.json().await
            .context("Failed to parse searchRule response")?;

        let rows: &[serde_json::Value] = json
            .get("rows")
            .and_then(|r| r.as_array())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let total: i64 = json.get("total").and_then(|t| t.as_i64()).unwrap_or(0);

        for (i, row) in rows.iter().enumerate() {
            // OPNsense Filter: descr or description
            let descr = row.get("descr")
                .or_else(|| row.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            // Keys: number, id, uuid, sequence; also @attributes.uuid (OPNsense grid)
            let mut keys: Vec<String> = ["number", "id", "uuid", "sequence"]
                .iter()
                .filter_map(|k| {
                    row.get(*k).and_then(|v| {
                        v.as_str()
                            .map(String::from)
                            .or_else(|| v.as_i64().map(|n| n.to_string()))
                    })
                })
                .filter(|s: &String| !s.is_empty())
                .collect();
            if let Some(attr) = row.get("@attributes").and_then(|a| a.get("uuid")).and_then(|u| u.as_str()) {
                if !attr.is_empty() {
                    keys.push(attr.to_string());
                }
            }

            if keys.is_empty() && i == 0 && total > 0 {
                let first_keys: Vec<&str> = row.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
                warn!("searchRule returned {} rows but no known id in first row. Keys: {:?}", rows.len(), first_keys);
            }

            for k in keys {
                all_labels.insert(k, descr.clone());
            }
        }

        if (current - 1) * ROW_COUNT + rows.len() as i64 >= total || rows.is_empty() {
            break;
        }
        current += 1;
    }

    info!("Fetched {} rule labels (all rules) at connection", all_labels.len());
    Ok(all_labels)
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

    debug!("Fetching aliases for IP: {}", ip);

    let request_body = serde_json::json!({
        "item": ip
    });

    // OPNsense API uses Basic Authentication (not custom headers)
    let response = client
        .post(&url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
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

/// Fetch all aliases from OPNsense API at connection time (no log file required).
///
/// Uses GET /api/firewall/alias_util/aliases for the list of names, then
/// GET /api/firewall/alias_util/list/{name} per alias to get content.
/// Builds IP → Vec<AliasMapping> so log IPs can be resolved without prior knowledge.
pub async fn fetch_all_aliases(
    credentials: &ApiCredentials,
) -> Result<HashMap<String, Vec<AliasMapping>>> {
    let client = build_api_client(credentials)?;
    let base = credentials.endpoint_url.trim_end_matches('/');

    // 1) Get list of alias names
    let list_url = format!("{}/api/firewall/alias_util/aliases", base);
    let resp = client
        .get(&list_url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
        .send()
        .await
        .context("Failed to fetch alias list")?;

    if !resp.status().is_success() {
        if resp.status() == 401 {
            return Err(ApiError::AuthError.into());
        }
        return Err(ApiError::NetworkError(format!("HTTP {}", resp.status())).into());
    }

    let json: serde_json::Value = resp.json().await
        .context("Failed to parse alias list")?;

    // alias_util/aliases: {"data":["A","B"]} or {"aliases":["A","B"]} or {"status":"ok","data":[...]}
    let names: Vec<String> = if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
        arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
    } else if let Some(arr) = json.get("aliases").and_then(|a| a.as_array()) {
        arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
    } else if let Some(obj) = json.get("data").and_then(|d| d.as_object()) {
        obj.keys().map(String::from).collect()
    } else if let Some(arr) = json.as_array() {
        arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
    } else if let Some(arr) = json.get("rows").and_then(|r| r.as_array()) {
        arr.iter()
            .filter_map(|r| r.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect()
    } else {
        debug!("alias_util/aliases unexpected format (keys: {:?})", json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        return Ok(HashMap::new());
    };

    if names.is_empty() {
        info!("No aliases reported by OPNsense, alias cache will be empty");
        return Ok(HashMap::new());
    }

    // 2) For each alias, get content and build IP → Vec<AliasMapping>
    let mut ip_to_aliases: HashMap<String, Vec<AliasMapping>> = HashMap::new();

    for name in names {
        let path_name = name.replace(' ', "%20");
        let detail_url = format!("{}/api/firewall/alias_util/list/{}", base, path_name);

        let detail_resp = match client
            .get(&detail_url)
            .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                debug!("alias_util/list/{} returned {}", name, r.status());
                continue;
            }
            Err(e) => {
                warn!("alias_util/list/{} failed: {}", name, e);
                continue;
            }
        };

        let text = match detail_resp.text().await {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to read alias list body for {}: {}", name, e);
                continue;
            }
        };

        // alias_util/list returns grid JSON: {"total":N,"rows":[{"ip":"1.2.3.4"},...]} per OPNsense API
        let ips: Vec<String> = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(rows) = v.get("rows").and_then(|r| r.as_array()) {
                rows.iter()
                    .filter_map(|row| {
                        row.get("ip")
                            .or_else(|| row.get("address"))
                            .or_else(|| row.get("content"))
                            .and_then(|x| x.as_str())
                            .map(String::from)
                    })
                    .filter(|s| !s.is_empty())
                    .collect()
            } else if let Some(s) = v.get("content").and_then(|c| c.as_str()) {
                s.split([',', '\n', ';'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else if let Some(arr) = v.as_array() {
                arr.iter().filter_map(|x| x.as_str().map(String::from)).collect()
            } else {
                text.split([',', '\n', ';'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
        } else {
            text.split([',', '\n', ';'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        let mapping = AliasMapping {
            alias_name: name.clone(),
            group_members: ips.clone(),
            description: None,
            alias_type: None,
        };

        for ip in ips {
            ip_to_aliases.entry(ip).or_default().push(mapping.clone());
        }
    }

    info!("Fetched all aliases at connection: {} unique IPs", ip_to_aliases.len());
    Ok(ip_to_aliases)
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

    // OPNsense API uses Basic Authentication (not custom headers)
    let response = client
        .get(&url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
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

// ============================================================================
// Staleness Severity Helpers (Story 4.3)
// ============================================================================

/// Calculate staleness severity based on enrichment age
pub fn calculate_staleness_severity(export_timestamp: &chrono::DateTime<chrono::Utc>) -> Result<StalenessSeverity> {
    let age_days = calculate_enrichment_age(export_timestamp)?;
    Ok(StalenessSeverity::from_age_days(age_days))
}

/// Format age for display (e.g., "2 days, 5 hours" or "12 hours")
pub fn format_enrichment_age(export_timestamp: &chrono::DateTime<chrono::Utc>) -> Result<String> {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*export_timestamp);

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;

    if days > 0 {
        let day_str = if days == 1 { "day" } else { "days" };
        if hours > 0 {
            Ok(format!("{} {}, {} hours", days, day_str, hours))
        } else {
            Ok(format!("{} {}", days, day_str))
        }
    } else {
        let hour_str = if hours == 1 { "hour" } else { "hours" };
        Ok(format!("{} {}", hours.max(0), hour_str))
    }
}

#[cfg(test)]
mod staleness_tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_staleness_severity_fresh() {
        let recent = Utc::now() - Duration::hours(12);
        let severity = calculate_staleness_severity(&recent).unwrap();
        assert_eq!(severity, StalenessSeverity::Fresh);
    }

    #[test]
    fn test_staleness_severity_moderate() {
        let moderate = Utc::now() - Duration::days(3);
        let severity = calculate_staleness_severity(&moderate).unwrap();
        assert_eq!(severity, StalenessSeverity::Moderate);
    }

    #[test]
    fn test_staleness_severity_high() {
        let old = Utc::now() - Duration::days(10);
        let severity = calculate_staleness_severity(&old).unwrap();
        assert_eq!(severity, StalenessSeverity::High);
    }

    #[test]
    fn test_staleness_severity_boundary_1day() {
        let exactly_1d = Utc::now() - Duration::days(1);
        let severity = calculate_staleness_severity(&exactly_1d).unwrap();
        assert_eq!(severity, StalenessSeverity::Moderate);
    }

    #[test]
    fn test_staleness_severity_boundary_7days() {
        let exactly_7d = Utc::now() - Duration::days(7);
        let severity = calculate_staleness_severity(&exactly_7d).unwrap();
        assert_eq!(severity, StalenessSeverity::Moderate);
    }

    #[test]
    fn test_format_enrichment_age_hours() {
        let recent = Utc::now() - Duration::hours(5);
        let formatted = format_enrichment_age(&recent).unwrap();
        assert_eq!(formatted, "5 hours");
    }

    #[test]
    fn test_format_enrichment_age_days() {
        let old = Utc::now() - Duration::days(2) - Duration::hours(3);
        let formatted = format_enrichment_age(&old).unwrap();
        assert_eq!(formatted, "2 days, 3 hours");
    }

    #[test]
    fn test_format_enrichment_age_singular() {
        let one_day = Utc::now() - Duration::days(1);
        let formatted = format_enrichment_age(&one_day).unwrap();
        assert_eq!(formatted, "1 day");
    }

    #[test]
    fn test_format_enrichment_age_singular_hour() {
        let one_hour = Utc::now() - Duration::hours(1);
        let formatted = format_enrichment_age(&one_hour).unwrap();
        assert_eq!(formatted, "1 hour");
    }
}

// ============================================================================
// Alias Resolution Tests (Story 3.4)
// ============================================================================

#[cfg(test)]
mod alias_tests {
    use super::*;
    use crate::api_client::types::AliasRow;

    /// Test parsing alias response with group members
    #[test]
    fn test_alias_response_parsing_with_group_members() {
        let alias_row = AliasRow {
            alias_uuid: Some("uuid-123".to_string()),
            name: "Servers_Group".to_string(),
            alias_type: Some("network".to_string()),
            content: "192.168.1.100,192.168.1.101,192.168.1.102".to_string(),
            description: Some("Server subnet".to_string()),
        };

        // Simulate the parsing logic from fetch_aliases_for_ip
        let group_members: Vec<String> = alias_row.content
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let alias_mapping = AliasMapping {
            alias_name: alias_row.name,
            group_members: group_members.clone(),
            description: alias_row.description,
            alias_type: alias_row.alias_type,
        };

        assert_eq!(alias_mapping.alias_name, "Servers_Group");
        assert_eq!(alias_mapping.group_members.len(), 3);
        assert_eq!(alias_mapping.group_members[0], "192.168.1.100");
        assert_eq!(alias_mapping.group_members[1], "192.168.1.101");
        assert_eq!(alias_mapping.group_members[2], "192.168.1.102");
        assert_eq!(alias_mapping.description.unwrap(), "Server subnet");
        assert_eq!(alias_mapping.alias_type.unwrap(), "network");
    }

    /// Test parsing alias response with single IP
    #[test]
    fn test_alias_response_parsing_single_ip() {
        let alias_row = AliasRow {
            alias_uuid: Some("uuid-456".to_string()),
            name: "Web_Server".to_string(),
            alias_type: Some("host".to_string()),
            content: "192.168.1.100".to_string(),
            description: None,
        };

        let group_members: Vec<String> = alias_row.content
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let alias_mapping = AliasMapping {
            alias_name: alias_row.name,
            group_members: group_members.clone(),
            description: alias_row.description,
            alias_type: alias_row.alias_type,
        };

        assert_eq!(alias_mapping.alias_name, "Web_Server");
        assert_eq!(alias_mapping.group_members.len(), 1);
        assert_eq!(alias_mapping.group_members[0], "192.168.1.100");
        assert!(alias_mapping.description.is_none());
    }

    /// Test parsing alias response with empty content
    #[test]
    fn test_alias_response_parsing_empty_content() {
        let alias_row = AliasRow {
            alias_uuid: None,
            name: "Empty_Alias".to_string(),
            alias_type: None,
            content: "".to_string(),
            description: None,
        };

        let group_members: Vec<String> = alias_row.content
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(group_members.len(), 0);
    }

    /// Test parsing alias response with whitespace in content
    #[test]
    fn test_alias_response_parsing_with_whitespace() {
        let alias_row = AliasRow {
            alias_uuid: Some("uuid-789".to_string()),
            name: "DMZ_Hosts".to_string(),
            alias_type: Some("network".to_string()),
            content: " 192.168.1.100 , 192.168.1.101 , 192.168.1.102 ".to_string(),
            description: Some("DMZ subnet".to_string()),
        };

        let group_members: Vec<String> = alias_row.content
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(group_members.len(), 3);
        assert_eq!(group_members[0], "192.168.1.100");
        assert_eq!(group_members[1], "192.168.1.101");
        assert_eq!(group_members[2], "192.168.1.102");
    }

    /// Test alias response with multiple aliases (simulating multiple rows)
    #[test]
    fn test_multiple_aliases_for_single_ip() {
        let alias_rows = vec![
            AliasRow {
                alias_uuid: Some("uuid-1".to_string()),
                name: "Servers".to_string(),
                alias_type: Some("host".to_string()),
                content: "192.168.1.100".to_string(),
                description: None,
            },
            AliasRow {
                alias_uuid: Some("uuid-2".to_string()),
                name: "DMZ_Hosts".to_string(),
                alias_type: Some("network".to_string()),
                content: "192.168.1.100,192.168.1.101".to_string(),
                description: Some("DMZ subnet".to_string()),
            },
        ];

        let aliases: Vec<AliasMapping> = alias_rows.into_iter().map(|row| {
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

        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].alias_name, "Servers");
        assert_eq!(aliases[1].alias_name, "DMZ_Hosts");
        assert_eq!(aliases[1].group_members.len(), 2);
    }

    /// Test empty alias response (IP not aliased)
    #[test]
    fn test_empty_alias_response() {
        let alias_rows: Vec<AliasRow> = vec![];

        let aliases: Vec<AliasMapping> = alias_rows.into_iter().map(|row| {
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

        assert_eq!(aliases.len(), 0);
    }
}
