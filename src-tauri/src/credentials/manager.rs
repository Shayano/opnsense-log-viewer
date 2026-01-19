use keyring::Entry;
use serde_json;
use crate::api_client::types::ApiCredentials;
use anyhow::{Result, Context};
use log::{info, debug, warn};

const SERVICE_NAME: &str = "opnsense-log-viewer";
const ACCOUNT_NAME: &str = "api-credentials";

/// Save credentials to OS keychain
pub fn save_credentials(credentials: &ApiCredentials) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
        .context("Failed to create keyring entry")?;

    let json = serde_json::to_string(credentials)
        .context("Failed to serialize credentials")?;

    entry.set_password(&json)
        .context("Failed to save credentials to keychain")?;

    info!("Credentials saved to OS keychain");
    Ok(())
}

/// Load credentials from OS keychain
pub fn load_credentials() -> Result<Option<ApiCredentials>> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
        .context("Failed to create keyring entry")?;

    match entry.get_password() {
        Ok(json) => {
            let credentials = serde_json::from_str(&json)
                .context("Failed to deserialize credentials")?;

            info!("Credentials loaded from OS keychain");
            Ok(Some(credentials))
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No credentials found in keychain");
            Ok(None)
        }
        Err(e) => {
            warn!("Failed to load credentials from keychain: {}", e);
            Err(e.into())
        }
    }
}

/// Delete credentials from OS keychain
pub fn delete_credentials() -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
        .context("Failed to create keyring entry")?;

    entry.delete_credential()
        .context("Failed to delete credentials from keychain")?;

    info!("Credentials deleted from OS keychain");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_credentials() {
        let creds = ApiCredentials {
            endpoint_url: "https://192.168.1.1".to_string(),
            api_key: "test-key".to_string(),
            api_secret: "test-secret".to_string(),
            profile_name: None,
            accept_invalid_certs: false,
        };

        // Save
        save_credentials(&creds).expect("Failed to save");

        // Load
        let loaded = load_credentials().expect("Failed to load");
        assert!(loaded.is_some());

        let loaded_creds = loaded.unwrap();
        assert_eq!(loaded_creds.endpoint_url, creds.endpoint_url);
        assert_eq!(loaded_creds.api_key, creds.api_key);

        // Cleanup
        delete_credentials().expect("Failed to delete");
    }

    #[test]
    fn test_load_nonexistent_credentials() {
        // Ensure no credentials exist
        let _ = delete_credentials();

        let result = load_credentials().expect("Failed to check credentials");
        assert!(result.is_none());
    }
}
