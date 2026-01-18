use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::task::JoinSet;

use crate::api_client::client::build_api_client;
use crate::api_client::types::{ApiCredentials, ApiError, InterfaceMappingResponse, RuleLabelResponse};

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

    log::debug!("Fetching interface mappings from OPNsense API");

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

    log::info!("Fetched {} interface mappings", normalized_mappings.len());
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

    log::debug!("Fetching rule label for hash: {}", rule_hash);

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
        .body(request_body.to_string())
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
    log::info!("Fetching {} rule labels in batch", rule_hashes.len());

    let mut tasks = JoinSet::new();
    let credentials = credentials.clone();

    // Spawn parallel tasks (tokio manages concurrency)
    for hash in rule_hashes {
        let credentials_clone = credentials.clone();
        let hash_clone = hash.clone();

        tasks.spawn(async move {
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
                log::debug!("Rule label not found for hash: {}", hash);
            }
            Ok((hash, Err(e))) => {
                log::warn!("Failed to fetch rule label for {}: {}", hash, e);
                error_count += 1;
            }
            Err(e) => {
                log::error!("Task join error: {}", e);
                error_count += 1;
            }
        }
    }

    log::info!("Rule label fetch complete: {} found, {} errors", success_count, error_count);
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
