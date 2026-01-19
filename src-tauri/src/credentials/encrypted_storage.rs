use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};
use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, rand_core::OsRng}};
use serde_json;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context, bail};
use crate::api_client::types::ApiCredentials;
use log::{info, debug};

const ENCRYPTED_FILE_NAME: &str = ".credentials.enc";

/// Get device-specific seed for key derivation
fn get_device_seed() -> Result<String> {
    // Use machine-uid for stable device identifier
    let machine_id = machine_uid::get()
        .map_err(|e| anyhow::anyhow!("Failed to get machine ID: {}", e))?;
    Ok(machine_id)
}

/// Derive encryption key from device seed using Argon2id
fn derive_key(device_seed: &str) -> Result<[u8; 32]> {
    // Use a fixed, valid Base64 salt for key derivation
    // In production, this provides consistent key derivation across sessions
    let salt = SaltString::from_b64("b3Buc2Vuc2UtbG9nLXZpZXdlci1zYWx0")
        .map_err(|e| anyhow::anyhow!("Invalid salt: {}", e))?;

    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(device_seed.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password with Argon2: {}", e))?;

    let hash = password_hash.hash
        .context("No hash produced")?;
    let hash_bytes = hash.as_bytes();

    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes[..32]);
    Ok(key)
}

/// Get encrypted file path
fn get_encrypted_file_path() -> Result<PathBuf> {
    // Use proper app data directory instead of current_dir
    let app_data_dir = dirs::data_local_dir()
        .context("Failed to get app data directory")?
        .join("opnsense-log-viewer");

    fs::create_dir_all(&app_data_dir)
        .context("Failed to create app data directory")?;

    Ok(app_data_dir.join(ENCRYPTED_FILE_NAME))
}

/// Encrypt and save credentials to file
pub fn encrypt_and_save(credentials: &ApiCredentials) -> Result<()> {
    let device_seed = get_device_seed()?;
    let key_bytes = derive_key(&device_seed)?;

    let json = serde_json::to_string(credentials)
        .context("Failed to serialize credentials")?;

    // Generate random nonce
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to generate random nonce"))?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    // Encrypt
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to create encryption key"))?;
    let mut sealing_key = SealingKey::new(unbound_key, OneNonceSequence(Some(nonce)));

    let mut ciphertext = json.as_bytes().to_vec();
    sealing_key.seal_in_place_append_tag(Aad::empty(), &mut ciphertext)
        .map_err(|_| anyhow::anyhow!("Failed to encrypt credentials"))?;

    // Prepend nonce to ciphertext
    let mut encrypted_data = nonce_bytes.to_vec();
    encrypted_data.extend_from_slice(&ciphertext);

    // Write to file
    let file_path = get_encrypted_file_path()?;
    fs::write(&file_path, encrypted_data)
        .context("Failed to write encrypted credentials")?;

    info!("Credentials saved to encrypted file (fallback)");
    Ok(())
}

/// Decrypt and load credentials from file
pub fn decrypt_and_load() -> Result<Option<ApiCredentials>> {
    let file_path = get_encrypted_file_path()?;

    if !file_path.exists() {
        debug!("No encrypted credentials file found");
        return Ok(None);
    }

    let encrypted_data = fs::read(&file_path)
        .context("Failed to read encrypted credentials file")?;

    if encrypted_data.len() < 12 {
        bail!("Encrypted file too short (corrupted)");
    }

    // Extract nonce and ciphertext
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let mut nonce_array = [0u8; 12];
    nonce_array.copy_from_slice(nonce_bytes);
    let nonce = Nonce::assume_unique_for_key(nonce_array);

    // Derive key
    let device_seed = get_device_seed()?;
    let key_bytes = derive_key(&device_seed)?;

    // Decrypt
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to create decryption key"))?;
    let mut opening_key = OpeningKey::new(unbound_key, OneNonceSequence(Some(nonce)));

    let mut plaintext = ciphertext.to_vec();
    let decrypted = opening_key.open_in_place(Aad::empty(), &mut plaintext)
        .map_err(|_| anyhow::anyhow!("Failed to decrypt credentials (corrupted or wrong key)"))?;

    let json = std::str::from_utf8(decrypted)
        .context("Invalid UTF-8 in decrypted data")?;
    let credentials = serde_json::from_str(json)
        .context("Failed to deserialize credentials")?;

    info!("Credentials loaded from encrypted file (fallback)");
    Ok(Some(credentials))
}

/// OneNonceSequence for single-use nonce
struct OneNonceSequence(Option<Nonce>);

impl NonceSequence for OneNonceSequence {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        self.0.take().ok_or(Unspecified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_and_decrypt() {
        let creds = ApiCredentials {
            endpoint_url: "https://192.168.1.1".to_string(),
            api_key: "test-key".to_string(),
            api_secret: "test-secret".to_string(),
            profile_name: None,
            accept_invalid_certs: false,
        };

        // Encrypt and save
        encrypt_and_save(&creds).expect("Failed to encrypt");

        // Decrypt and load
        let loaded = decrypt_and_load().expect("Failed to decrypt");
        assert!(loaded.is_some());

        let loaded_creds = loaded.unwrap();
        assert_eq!(loaded_creds.endpoint_url, creds.endpoint_url);
        assert_eq!(loaded_creds.api_key, creds.api_key);

        // Cleanup
        let file_path = get_encrypted_file_path().unwrap();
        if file_path.exists() {
            fs::remove_file(file_path).expect("Failed to cleanup");
        }
    }

    #[test]
    fn test_decrypt_nonexistent_file() {
        // Ensure file doesn't exist
        let file_path = get_encrypted_file_path().unwrap();
        if file_path.exists() {
            fs::remove_file(&file_path).ok();
        }

        let result = decrypt_and_load().expect("Failed to check for file");
        assert!(result.is_none());
    }
}
