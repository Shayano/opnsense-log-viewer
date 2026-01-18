use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use std::time::Duration;
use crate::api_client::types::{ApiCredentials, ApiError, ConnectionTestResult};
use anyhow::{Result, Context};
use log::{info, debug};

/// Build API client with retry middleware
pub fn build_api_client(credentials: &ApiCredentials) -> Result<ClientWithMiddleware> {
    // Enforce HTTPS
    if !credentials.endpoint_url.starts_with("https://") {
        return Err(ApiError::HttpNotAllowed.into());
    }

    // Configure retry policy: 100ms → 200ms → 400ms → 800ms → 1600ms (3 retries)
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_millis(1600))
        .build_with_max_retries(3);

    // Build reqwest client
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to build HTTP client")?;

    // Wrap with retry middleware
    let client_with_middleware = ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    Ok(client_with_middleware)
}

/// Test connection to OPNsense API
pub async fn test_connection(credentials: &ApiCredentials) -> Result<ConnectionTestResult> {
    let client = build_api_client(credentials)?;

    let url = format!("{}/api/diagnostics/interface/getInterfaceNames", credentials.endpoint_url);

    // Security: Do NOT log endpoint URL (may contain sensitive info)
    debug!("Testing API connection");

    // Make test API call
    let response = client
        .get(&url)
        .header("X-API-Key", &credentials.api_key)
        .header("X-API-Secret", &credentials.api_secret)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let interfaces: serde_json::Value = resp.json().await
                    .context("Failed to parse interface response")?;

                let interface_count = interfaces.as_object()
                    .map(|obj| obj.len())
                    .unwrap_or(0);

                // Optional: Fetch OPNsense version
                let version = fetch_opnsense_version(&client, credentials).await.ok();

                info!("Connection test successful: {} interfaces found", interface_count);

                Ok(ConnectionTestResult {
                    success: true,
                    interface_count,
                    opnsense_version: version,
                    error_message: None,
                })
            } else if resp.status() == 401 {
                Err(ApiError::AuthError.into())
            } else {
                Err(ApiError::NetworkError(format!("HTTP {}", resp.status())).into())
            }
        }
        Err(e) => {
            if e.is_timeout() {
                Err(ApiError::TimeoutError(10).into())
            } else if e.is_connect() {
                Err(ApiError::NetworkError("Connection refused".to_string()).into())
            } else if e.to_string().contains("certificate") {
                Err(ApiError::TlsError(e.to_string()).into())
            } else {
                Err(ApiError::NetworkError(e.to_string()).into())
            }
        }
    }
}

/// Fetch OPNsense version (optional)
async fn fetch_opnsense_version(
    client: &ClientWithMiddleware,
    credentials: &ApiCredentials,
) -> Result<String> {
    let url = format!("{}/api/core/firmware/status", credentials.endpoint_url);

    let response = client
        .get(&url)
        .header("X-API-Key", &credentials.api_key)
        .header("X-API-Secret", &credentials.api_secret)
        .send()
        .await?;

    let status: serde_json::Value = response.json().await?;
    let version = status["product_version"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_https_enforcement() {
        let creds = ApiCredentials {
            endpoint_url: "http://192.168.1.1".to_string(), // HTTP (should fail)
            api_key: "test".to_string(),
            api_secret: "test".to_string(),
            profile_name: None,
        };

        let result = build_api_client(&creds);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_client_with_https() {
        let creds = ApiCredentials {
            endpoint_url: "https://192.168.1.1".to_string(),
            api_key: "test".to_string(),
            api_secret: "test".to_string(),
            profile_name: None,
        };

        let result = build_api_client(&creds);
        assert!(result.is_ok());
    }
}
