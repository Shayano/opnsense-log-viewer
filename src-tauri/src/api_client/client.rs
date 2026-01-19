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
    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(10));
    
    // Accept invalid certificates if requested (for self-signed certificates)
    if credentials.accept_invalid_certs {
        client_builder = client_builder.danger_accept_invalid_certs(true);
    }
    
    let client = client_builder
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

    // Log URL structure (without credentials) for debugging port/endpoint issues
    // Parse URL to show just the host and port
    if let Ok(parsed_url) = reqwest::Url::parse(&credentials.endpoint_url) {
        let host_port = if let Some(port) = parsed_url.port() {
            format!("{}:{}", parsed_url.host_str().unwrap_or("unknown"), port)
        } else {
            parsed_url.host_str().unwrap_or("unknown").to_string()
        };
        debug!("Testing API connection to {} (scheme: {})", host_port, parsed_url.scheme());
    } else {
        debug!("Testing API connection (URL parsing failed)");
    }

    // Make test API call
    // OPNsense API uses Basic Authentication (not custom headers)
    // See: https://docs.opnsense.org/development/how-tos/api.html
    let response = client
        .get(&url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                // Read response text first for better error messages
                let response_text = resp.text().await
                    .context("Failed to read response body")?;
                
                // Check if response is HTML instead of JSON (common OPNsense error)
                let response_trimmed = response_text.trim();
                if response_trimmed.starts_with("<!doctype") 
                    || response_trimmed.starts_with("<!DOCTYPE")
                    || response_trimmed.starts_with("<html")
                    || response_trimmed.starts_with("<HTML") {
                    return Err(ApiError::InvalidResponse(format!(
                        "API returned HTML instead of JSON. This usually means:\n\
                        1. The endpoint URL is incorrect or incomplete\n\
                        2. Authentication failed and you were redirected to a login page\n\
                        3. The API endpoint doesn't exist on this OPNsense instance\n\
                        4. The port number might be incorrect (OPNsense API uses the same port as the web interface, typically 443 or 80)\n\
                        \n\
                        Verify that:\n\
                        - The endpoint URL is correct (e.g., https://192.168.1.1 or https://192.168.1.1:443)\n\
                        - The API key and secret are valid\n\
                        - The OPNsense API is enabled in System > Settings > API\n\
                        - You're using the correct port (usually the same as the web interface)\n\
                        \n\
                        Response preview: {}",
                        if response_text.len() > 500 {
                            format!("{}...", &response_text[..500])
                        } else {
                            response_text.clone()
                        }
                    )).into());
                }
                
                // Log raw response for debugging (truncated to avoid sensitive data)
                let preview = if response_text.len() > 200 {
                    format!("{}...", &response_text[..200])
                } else {
                    response_text.clone()
                };
                debug!("API response preview: {}", preview);

                // Try to parse as JSON
                let interfaces: serde_json::Value = serde_json::from_str(&response_text)
                    .with_context(|| format!(
                        "Failed to parse interface response as JSON. Response: {}",
                        if response_text.len() > 500 {
                            format!("{}... (truncated)", &response_text[..500])
                        } else {
                            response_text.clone()
                        }
                    ))?;

                // Handle different OPNsense response formats
                let interface_count = if let Some(obj) = interfaces.as_object() {
                    // Direct format: {"vtnet0": "lan", "vtnet1": "wan"}
                    obj.len()
                } else if let Some(items) = interfaces.get("items") {
                    // Nested format: {"items": {"vtnet0": "lan"}}
                    items.as_object().map(|o| o.len()).unwrap_or(0)
                } else if let Some(data) = interfaces.get("data") {
                    // Data wrapper format: {"data": {"vtnet0": "lan"}}
                    data.as_object().map(|o| o.len()).unwrap_or(0)
                } else {
                    debug!("Unexpected response format: {}", interfaces);
                    0
                };

                // Optional: Fetch OPNsense version
                let version = fetch_opnsense_version(&client, credentials).await.ok();

                info!("Connection test successful: {} interfaces found", interface_count);

                Ok(ConnectionTestResult {
                    success: true,
                    interface_count,
                    opnsense_version: version,
                    error_message: None,
                })
            } else {
                // Save status before consuming response
                let status = resp.status();
                let status_code = status.as_u16();
                
                // Try to read error message from response body
                let error_body = resp.text().await.unwrap_or_default();
                let error_msg = if error_body.is_empty() {
                    format!("HTTP {}", status_code)
                } else {
                    format!("HTTP {}: {}", status_code, 
                        if error_body.len() > 200 {
                            format!("{}...", &error_body[..200])
                        } else {
                            error_body
                        })
                };
                
                if status == 401 {
                    Err(ApiError::AuthError.into())
                } else {
                    Err(ApiError::NetworkError(error_msg).into())
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            if e.is_timeout() {
                Err(ApiError::TimeoutError(10).into())
            } else if e.is_connect() {
                Err(ApiError::NetworkError("Connection refused".to_string()).into())
            } else if error_msg.contains("certificate") 
                || error_msg.contains("certificate verify failed")
                || error_msg.contains("invalid peer certificate")
                || error_msg.contains("unable to get local issuer certificate")
                || error_msg.contains("self signed certificate")
                || error_msg.contains("certificate signed by unknown authority") {
                Err(ApiError::TlsError(format!(
                    "TLS certificate validation failed: {}. Enable 'Accept Invalid Certificates' option if using a self-signed certificate.",
                    error_msg
                )).into())
            } else {
                Err(ApiError::NetworkError(error_msg).into())
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

    // OPNsense API uses Basic Authentication (not custom headers)
    let response = client
        .get(&url)
        .basic_auth(&credentials.api_key, Some(&credentials.api_secret))
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
            accept_invalid_certs: false,
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
            accept_invalid_certs: false,
        };

        let result = build_api_client(&creds);
        assert!(result.is_ok());
    }
}
