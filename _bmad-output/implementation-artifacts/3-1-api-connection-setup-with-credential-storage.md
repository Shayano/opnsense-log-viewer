# Story 3.1: API Connection Setup with Credential Storage

Status: ready-for-dev

## Story

As a network administrator,
I want to configure my OPNsense API connection with secure credential storage,
So that I can connect once and have my credentials remembered securely across sessions.

## Acceptance Criteria

**Given** the application is running
**When** I open Settings > OPNsense API Configuration
**Then** I see a configuration form with fields:
- Endpoint URL (placeholder: "https://192.168.1.1")
- API Key (password field with show/hide toggle)
- API Secret (password field with show/hide toggle)
- [Test Connection] button
- [Save] button
- Connection status indicator (initially: Disconnected - Red)

**When** I enter API credentials
**Then** input validation ensures:
- Endpoint URL uses HTTPS protocol (HTTP rejected per NFR-003.4)
- Endpoint URL is a valid URL format
- API Key and Secret are non-empty strings

**When** I click [Test Connection]
**Then** the application attempts to connect to the OPNsense API
**And** makes a test call to `/api/diagnostics/interface/getInterfaceNames`
**And** a loading spinner appears during the test

**When** the connection succeeds
**Then** a success message displays: "Connected to OPNsense [version]. Found X interfaces."
**And** connection status indicator changes to: Connected - Green
**And** OPNsense version is displayed

**When** the connection fails
**Then** an error message displays with specific reason:
- "Connection refused: Unable to reach [URL]. Check firewall and network settings."
- "Authentication failed: Invalid API key or secret. Verify credentials in OPNsense."
- "TLS certificate invalid: [details]. Enable 'Accept Invalid Certificates' in Advanced settings (not recommended)."
**And** connection status indicator remains: Disconnected - Red

**When** I click [Save] with valid credentials
**Then** credentials are stored securely using OS keychain per NFR-003.1:
- Windows: Windows Credential Manager
- macOS: macOS Keychain
- Linux: Secret Service (libsecret)

**And** if OS keychain is unavailable
**Then** fallback to encrypted file storage:
- AES-256-GCM encryption
- Argon2id key derivation from device-specific seed
- Encrypted file stored in application data directory

**And** credentials never appear in:
- Application logs
- Error messages
- Debug output
- Network traffic logs (only sent via HTTPS to user's specified OPNsense endpoint)

**When** I reopen the application
**Then** saved credentials are automatically loaded from OS keychain
**And** connection status indicator shows: Attempting connection...
**And** automatic connection attempt is made in background
**And** if successful, status changes to: Connected - Green

**When** I want to manage multiple OPNsense devices
**Then** I can switch between saved device profiles:
- Profile name (e.g., "Office Firewall", "Home OPNsense")
- Each profile stores its own endpoint + credentials
- Active profile indicated in UI

## Tasks / Subtasks

- [ ] Create API credential types and data structures (AC: Type system)
  - [ ] Define ApiCredentials interface in src-tauri/src/api_client/types.rs
  - [ ] Fields: endpoint_url, api_key, api_secret, profile_name (optional)
  - [ ] Define ConnectionStatus enum: Disconnected, Connecting, Connected, Error
  - [ ] Define ApiResponse types for test connection

- [ ] Implement OS keychain integration (AC: Secure credential storage)
  - [ ] Create src-tauri/src/credentials/manager.rs
  - [ ] Add keyring 3.6.3 dependency to Cargo.toml
  - [ ] Implement save_credentials() using keyring::Entry
  - [ ] Service name: "opnsense-log-viewer"
  - [ ] Account name: "api-credentials" (single profile MVP)
  - [ ] Implement load_credentials() from OS keychain
  - [ ] Implement delete_credentials()
  - [ ] Serialize credentials as JSON with serde

- [ ] Implement AES-256-GCM encrypted fallback storage (AC: Fallback encryption)
  - [ ] Create src-tauri/src/credentials/encrypted_storage.rs
  - [ ] Use rust-crypto or ring crate for AES-256-GCM
  - [ ] Use argon2 crate for Argon2id key derivation
  - [ ] Device seed generation (machine-specific, stable)
  - [ ] Encrypted file location: app data dir / .credentials.enc
  - [ ] Implement encrypt_and_save() and decrypt_and_load()
  - [ ] Handle corruption gracefully (prompt for re-entry)

- [ ] Create OPNsense API client with retry logic (AC: HTTP client + test connection)
  - [ ] Create src-tauri/src/api_client/client.rs
  - [ ] Add reqwest 0.13.1 with rustls-tls feature to Cargo.toml
  - [ ] Add reqwest-middleware 0.3 and reqwest-retry 0.6
  - [ ] Configure exponential backoff: 100ms → 1600ms, max 3 retries
  - [ ] Configure timeout: 10 seconds per request
  - [ ] Implement build_api_client(credentials: ApiCredentials) -> ClientWithMiddleware
  - [ ] Add API key/secret to request headers (X-API-Key, X-API-Secret)
  - [ ] Enforce HTTPS only (reject HTTP endpoints)

- [ ] Implement test connection endpoint (AC: Test connection functionality)
  - [ ] Create test_connection() function in client.rs
  - [ ] Call GET /api/diagnostics/interface/getInterfaceNames
  - [ ] Parse response to count interfaces
  - [ ] Optional: Call GET /api/core/firmware/status for version detection
  - [ ] Return Result<ConnectionTestResult, ApiError>
  - [ ] ConnectionTestResult: success, interface_count, opnsense_version (optional)
  - [ ] ApiError variants: NetworkError, AuthError, TlsError, TimeoutError

- [ ] Create Tauri command for saving credentials (AC: IPC commands)
  - [ ] Create #[tauri::command] save_api_credentials(endpoint, api_key, api_secret)
  - [ ] Validate inputs (non-empty, HTTPS enforcement)
  - [ ] Try keyring::Entry first
  - [ ] Fallback to encrypted storage if keyring fails
  - [ ] Return Result<(), String> to frontend
  - [ ] Never log credentials in success or error paths

- [ ] Create Tauri command for loading credentials (AC: IPC commands)
  - [ ] Create #[tauri::command] load_api_credentials() -> Option<ApiCredentials>
  - [ ] Try keyring::Entry first
  - [ ] Fallback to encrypted storage if keyring empty
  - [ ] Return Option (None if no credentials saved)
  - [ ] Strip sensitive data from logs

- [ ] Create Tauri command for test connection (AC: IPC commands)
  - [ ] Create #[tauri::command] test_api_connection(endpoint, api_key, api_secret)
  - [ ] Build API client with provided credentials
  - [ ] Call test_connection() function
  - [ ] Return Result<ConnectionTestResult, String>
  - [ ] Format errors with user-friendly messages per AC

- [ ] Create ApiConfigSettings React component (AC: UI form)
  - [ ] Create src/components/settings/api-config-settings.tsx
  - [ ] Use React Hook Form 7.x for form management
  - [ ] Endpoint URL input with HTTPS validation
  - [ ] API Key password input with show/hide toggle (Eye icon)
  - [ ] API Secret password input with show/hide toggle
  - [ ] Test Connection button with loading spinner
  - [ ] Save button (disabled until valid input)
  - [ ] Connection status indicator (colored dot + text)

- [ ] Implement connection status indicator (AC: Visual feedback)
  - [ ] Create ConnectionStatusBadge component
  - [ ] Disconnected: Red dot + "Disconnected"
  - [ ] Connecting: Yellow dot + spinning icon + "Connecting..."
  - [ ] Connected: Green dot + "Connected to [hostname]"
  - [ ] Error: Red dot + "Connection Failed"
  - [ ] Use lucide-react icons: Wifi, WifiOff, Loader2

- [ ] Implement Test Connection functionality (AC: Frontend integration)
  - [ ] On Test Connection click, call invoke('test_api_connection', { endpoint, apiKey, apiSecret })
  - [ ] Show loading spinner during test
  - [ ] On success: Update status to Connected, show toast with interface count
  - [ ] On failure: Update status to Error, show toast with specific error message
  - [ ] Display OPNsense version if available

- [ ] Implement Save functionality (AC: Frontend integration)
  - [ ] On Save click, call invoke('save_api_credentials', { endpoint, apiKey, apiSecret })
  - [ ] Show loading spinner during save
  - [ ] On success: Show toast "Credentials saved securely"
  - [ ] On error: Show toast with specific error (e.g., "Keychain unavailable, using encrypted fallback")
  - [ ] Clear password fields from memory after save

- [ ] Implement auto-load on app startup (AC: Automatic credential loading)
  - [ ] In App.tsx or main layout component useEffect
  - [ ] Call invoke('load_api_credentials') on mount
  - [ ] If credentials exist, populate form fields (but keep passwords hidden by default)
  - [ ] Automatically attempt connection in background
  - [ ] Update connection status indicator based on result
  - [ ] Silent failure if no credentials (first launch)

- [ ] Add HTTPS enforcement validation (AC: Security requirement NFR-003.4)
  - [ ] Frontend: Reject HTTP URLs with error message
  - [ ] Backend: Reject HTTP URLs in save_api_credentials and test_api_connection
  - [ ] Error message: "HTTPS required for security. Use https:// instead of http://"
  - [ ] Allow localhost HTTP only for development (optional feature flag)

- [ ] Integrate Settings panel into main app (AC: UI integration)
  - [ ] Create Settings page/dialog (Settings icon in toolbar)
  - [ ] Add "OPNsense API Configuration" section
  - [ ] Include ApiConfigSettings component
  - [ ] Maintain dark/light theme support
  - [ ] Keyboard navigation: Tab through fields, Enter to Test/Save

- [ ] Write unit tests - Credential manager (AC: Backend testing)
  - [ ] Test save_credentials to keyring
  - [ ] Test load_credentials from keyring
  - [ ] Test delete_credentials
  - [ ] Test keyring failure → encrypted fallback
  - [ ] Test serialization/deserialization
  - [ ] Mock keyring::Entry for deterministic tests
  - [ ] Achieve 85%+ coverage

- [ ] Write unit tests - Encrypted storage fallback (AC: Backend testing)
  - [ ] Test encrypt_and_save with AES-256-GCM
  - [ ] Test decrypt_and_load
  - [ ] Test Argon2id key derivation
  - [ ] Test file corruption handling
  - [ ] Test device seed generation stability
  - [ ] Achieve 85%+ coverage

- [ ] Write unit tests - API client (AC: Backend testing)
  - [ ] Test build_api_client configuration
  - [ ] Test HTTPS enforcement (reject HTTP)
  - [ ] Test retry logic with mock server (3 retries, exponential backoff)
  - [ ] Test timeout after 10 seconds
  - [ ] Test API key/secret header injection
  - [ ] Mock reqwest for deterministic tests
  - [ ] Achieve 80%+ coverage

- [ ] Write unit tests - Test connection endpoint (AC: Backend testing)
  - [ ] Test successful connection with mock OPNsense API
  - [ ] Test authentication failure (401)
  - [ ] Test network error (connection refused)
  - [ ] Test TLS certificate error
  - [ ] Test timeout
  - [ ] Test interface count parsing
  - [ ] Achieve 85%+ coverage

- [ ] Write unit tests - ApiConfigSettings component (AC: Frontend testing)
  - [ ] Test rendering with empty form
  - [ ] Test rendering with loaded credentials
  - [ ] Test HTTPS validation
  - [ ] Test non-empty validation
  - [ ] Test Test Connection button behavior
  - [ ] Test Save button behavior
  - [ ] Test show/hide password toggle
  - [ ] Test connection status indicator updates
  - [ ] Achieve 80%+ coverage

- [ ] Write integration tests (AC: End-to-end workflow)
  - [ ] Test: Enter credentials → Test Connection → Success → Save
  - [ ] Test: Save credentials → Restart app → Auto-load → Auto-connect
  - [ ] Test: Invalid credentials → Test Connection → Error message
  - [ ] Test: Keyring unavailable → Fallback to encrypted storage
  - [ ] Test: HTTPS enforcement (reject HTTP URLs)
  - [ ] Test: Connection timeout handling

- [ ] Security audit (AC: NFR-003.1, NFR-003.4)
  - [ ] Verify credentials never appear in logs
  - [ ] Verify credentials never appear in error messages
  - [ ] Verify HTTPS enforcement (no HTTP allowed)
  - [ ] Verify AES-256-GCM encryption correctness
  - [ ] Verify Argon2id key derivation parameters
  - [ ] Verify no credential leakage in debug mode
  - [ ] Test with security scanner (cargo audit)

- [ ] Performance testing (AC: No UI blocking)
  - [ ] Test Test Connection completes in <10 seconds (timeout)
  - [ ] Test Save credentials completes in <500ms
  - [ ] Test Load credentials completes in <200ms
  - [ ] Test auto-connect on startup doesn't block UI (<3 seconds background)
  - [ ] Verify no memory leaks with repeated save/load

## Dev Notes

### 🔥 ULTIMATE STORY CONTEXT ENGINE OUTPUT 🔥

This story file has been generated by the comprehensive context analysis engine. It contains EVERYTHING you need to implement Story 3.1 correctly, aligned with architecture, Epic 3 requirements, and the established codebase conventions.

---

### Architecture Compliance & Technical Requirements

#### **Technology Stack (MUST USE EXACT VERSIONS)**

**Backend - Credential Storage & API Client:**
- **keyring 3.6.3** - OS keychain integration
  - Features: `["apple-native", "windows-native", "sync-secret-service"]`
  - Windows: Windows Credential Manager
  - macOS: macOS Keychain
  - Linux: Secret Service (libsecret)
- **reqwest 0.13.1** - HTTP client for OPNsense API
  - Features: `["json", "rustls-tls"]`
  - TLS via rustls (cross-platform, no OpenSSL dependency)
- **reqwest-middleware 0.3** - Retry middleware
- **reqwest-retry 0.6** - Exponential backoff retry policy
- **serde 1.x** + **serde_json 1.x** - Credential serialization
- **ring** or **rust-crypto** - AES-256-GCM encryption for fallback
- **argon2 3.x** - Argon2id key derivation
- **tokio 1.x** - Async runtime (already configured)
- **thiserror 2.x** - Error types

**Frontend - Settings UI:**
- **React 18.3+** (already installed) - Component framework
- **React Hook Form 7.x** (already installed) - Form validation
- **Zustand 5.0.10** (already installed) - Optional: API connection state
- **TypeScript 5.7** (already installed) - Type-safe API client
- **lucide-react** (already installed) - Icons
  - ✅ Wifi (connected), WifiOff (disconnected), Loader2 (connecting), Eye/EyeOff (password toggle), Settings (settings icon)
- **Tailwind CSS 3.4+** (already configured) - Styling
- **react-hot-toast 2.4+** (already installed) - User feedback

**Performance Requirements:**
- Test connection: ≤10 seconds (timeout)
- Save credentials: <500ms
- Load credentials: <200ms
- Auto-connect on startup: <3 seconds (background, non-blocking)

**Quality Gates:**
- Backend test coverage: 85%+ (credentials, API client)
- Frontend test coverage: 80%+ (components)
- Security audit passes (cargo audit, no credential leaks)
- HTTPS enforcement verified

---

#### **Code Structure & File Organization**

**Backend Structure (NEW files for Story 3.1):**
```
src-tauri/
├── src/
│   ├── api_client/                      # NEW - OPNsense API client
│   │   ├── mod.rs                       # Module exports
│   │   ├── client.rs                    # NEW - HTTP client with retry logic
│   │   ├── types.rs                     # NEW - ApiCredentials, ConnectionStatus types
│   │   ├── commands.rs                  # NEW - Tauri commands for IPC
│   │   └── client.test.rs               # NEW - Unit tests
│   ├── credentials/                     # NEW - Credential management
│   │   ├── mod.rs                       # Module exports
│   │   ├── manager.rs                   # NEW - OS keychain integration
│   │   ├── encrypted_storage.rs         # NEW - AES-256-GCM fallback
│   │   ├── manager.test.rs              # NEW - Unit tests
│   │   └── encrypted_storage.test.rs    # NEW - Unit tests
│   └── main.rs                          # MODIFY - Register Tauri commands
└── Cargo.toml                           # MODIFY - Add dependencies
```

**Frontend Structure (NEW files for Story 3.1):**
```
src/
├── components/
│   ├── settings/
│   │   ├── api-config-settings.tsx      # NEW - API configuration form
│   │   ├── connection-status-badge.tsx  # NEW - Status indicator
│   │   ├── api-config-settings.test.tsx # NEW - Component tests
│   │   └── index.ts                     # NEW - Exports
│   └── settings-dialog.tsx              # NEW - Settings dialog/page
├── stores/
│   └── api-connection-store.ts          # NEW - Optional: Connection state
├── types/
│   └── api.ts                           # NEW - TypeScript API types
└── utils/
    └── api-client.ts                    # NEW - Tauri invoke wrappers
```

---

#### **Cargo.toml Dependencies**

**Add to src-tauri/Cargo.toml:**

```toml
[dependencies]
# Existing dependencies (already in project)
tauri = { version = "2.x", features = ["..." ] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "fs", "io-util", "time"] }
thiserror = "2"
anyhow = "1"

# NEW: HTTP client with retry logic
reqwest = { version = "0.13", features = ["json", "rustls-tls"] }
reqwest-middleware = "0.3"
reqwest-retry = "0.6"

# NEW: OS keychain integration
keyring = { version = "3.6", features = ["apple-native", "windows-native", "sync-secret-service"] }

# NEW: Encrypted fallback storage
ring = "0.17"  # For AES-256-GCM encryption
argon2 = "0.5" # For Argon2id key derivation

# Optional: For device fingerprinting (machine-specific seed)
machine-uid = "0.5"
```

---

#### **Backend Type Definitions**

**File: src-tauri/src/api_client/types.rs (NEW)**

```rust
use serde::{Deserialize, Serialize};

/// API credentials for OPNsense connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCredentials {
    pub endpoint_url: String,
    pub api_key: String,
    pub api_secret: String,
    #[serde(default)]
    pub profile_name: Option<String>, // For future multi-profile support
}

/// Connection status for UI indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Result from test connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub interface_count: usize,
    pub opnsense_version: Option<String>,
    pub error_message: Option<String>,
}

/// API error types
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication failed: Invalid API key or secret")]
    AuthError,

    #[error("TLS certificate error: {0}")]
    TlsError(String),

    #[error("Request timeout after {0} seconds")]
    TimeoutError(u64),

    #[error("HTTP endpoint not allowed (HTTPS required)")]
    HttpNotAllowed,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}
```

---

#### **OS Keychain Integration**

**File: src-tauri/src/credentials/manager.rs (NEW)**

```rust
use keyring::Entry;
use serde_json;
use crate::api_client::types::ApiCredentials;
use anyhow::{Result, Context};

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

    tracing::info!("Credentials saved to OS keychain");
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

            tracing::info!("Credentials loaded from OS keychain");
            Ok(Some(credentials))
        }
        Err(keyring::Error::NoEntry) => {
            tracing::debug!("No credentials found in keychain");
            Ok(None)
        }
        Err(e) => {
            tracing::warn!("Failed to load credentials from keychain: {}", e);
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

    tracing::info!("Credentials deleted from OS keychain");
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
}
```

---

#### **Encrypted Fallback Storage**

**File: src-tauri/src/credentials/encrypted_storage.rs (NEW)**

```rust
use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM};
use ring::error::Unspecified;
use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, rand_core::OsRng}};
use serde_json;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context, bail};
use crate::api_client::types::ApiCredentials;

const ENCRYPTED_FILE_NAME: &str = ".credentials.enc";

/// Get device-specific seed for key derivation
fn get_device_seed() -> Result<String> {
    // Use machine-uid crate for stable device identifier
    let machine_id = machine_uid::get()
        .context("Failed to get machine ID")?;
    Ok(machine_id)
}

/// Derive encryption key from device seed using Argon2id
fn derive_key(device_seed: &str) -> Result<[u8; 32]> {
    let salt = SaltString::from_b64("opnsense-log-viewer-salt")
        .context("Invalid salt")?;

    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(device_seed.as_bytes(), &salt)
        .context("Failed to hash password with Argon2")?;

    let hash_bytes = password_hash.hash
        .context("No hash produced")?
        .as_bytes();

    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes[..32]);
    Ok(key)
}

/// Get encrypted file path
fn get_encrypted_file_path() -> Result<PathBuf> {
    let app_data_dir = tauri::api::path::app_data_dir(&tauri::Config::default())
        .context("Failed to get app data directory")?;

    Ok(app_data_dir.join(ENCRYPTED_FILE_NAME))
}

/// Encrypt and save credentials to file
pub fn encrypt_and_save(credentials: &ApiCredentials) -> Result<()> {
    let device_seed = get_device_seed()?;
    let key_bytes = derive_key(&device_seed)?;

    let json = serde_json::to_string(credentials)
        .context("Failed to serialize credentials")?;

    // Generate random nonce
    let nonce_bytes = ring::rand::generate::<[u8; 12]>(&ring::rand::SystemRandom::new())
        .context("Failed to generate nonce")?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    // Encrypt
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .context("Failed to create encryption key")?;
    let mut sealing_key = SealingKey::new(unbound_key, OneNonceSequence(Some(nonce)));

    let mut ciphertext = json.as_bytes().to_vec();
    sealing_key.seal_in_place_append_tag(Aad::empty(), &mut ciphertext)
        .context("Failed to encrypt credentials")?;

    // Prepend nonce to ciphertext
    let mut encrypted_data = nonce_bytes.to_vec();
    encrypted_data.extend_from_slice(&ciphertext);

    // Write to file
    let file_path = get_encrypted_file_path()?;
    fs::create_dir_all(file_path.parent().unwrap())
        .context("Failed to create app data directory")?;
    fs::write(&file_path, encrypted_data)
        .context("Failed to write encrypted credentials")?;

    tracing::info!("Credentials saved to encrypted file (fallback)");
    Ok(())
}

/// Decrypt and load credentials from file
pub fn decrypt_and_load() -> Result<Option<ApiCredentials>> {
    let file_path = get_encrypted_file_path()?;

    if !file_path.exists() {
        tracing::debug!("No encrypted credentials file found");
        return Ok(None);
    }

    let encrypted_data = fs::read(&file_path)
        .context("Failed to read encrypted credentials file")?;

    if encrypted_data.len() < 12 {
        bail!("Encrypted file too short (corrupted)");
    }

    // Extract nonce and ciphertext
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .context("Invalid nonce")?;

    // Derive key
    let device_seed = get_device_seed()?;
    let key_bytes = derive_key(&device_seed)?;

    // Decrypt
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .context("Failed to create decryption key")?;
    let mut opening_key = OpeningKey::new(unbound_key, OneNonceSequence(Some(nonce)));

    let mut plaintext = ciphertext.to_vec();
    let decrypted = opening_key.open_in_place(Aad::empty(), &mut plaintext)
        .context("Failed to decrypt credentials (corrupted or wrong key)")?;

    let json = std::str::from_utf8(decrypted)
        .context("Invalid UTF-8 in decrypted data")?;
    let credentials = serde_json::from_str(json)
        .context("Failed to deserialize credentials")?;

    tracing::info!("Credentials loaded from encrypted file (fallback)");
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
}
```

---

#### **OPNsense API Client with Retry Logic**

**File: src-tauri/src/api_client/client.rs (NEW)**

```rust
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use std::time::Duration;
use crate::api_client::types::{ApiCredentials, ApiError, ConnectionTestResult};
use anyhow::{Result, Context};

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
```

---

#### **Tauri Commands for IPC**

**File: src-tauri/src/api_client/commands.rs (NEW)**

```rust
use tauri::command;
use crate::api_client::types::{ApiCredentials, ConnectionTestResult};
use crate::api_client::client::test_connection;
use crate::credentials::{manager, encrypted_storage};

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
            tracing::info!("Credentials saved to OS keychain");
            Ok(())
        }
        Err(e) => {
            // Fallback to encrypted file storage
            tracing::warn!("Keychain unavailable: {}. Using encrypted fallback.", e);
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
            tracing::info!("Credentials loaded from OS keychain");
            Ok(Some(creds))
        }
        Ok(None) | Err(_) => {
            // Fallback to encrypted file storage
            tracing::debug!("Keychain empty or unavailable. Trying encrypted fallback.");
            encrypted_storage::decrypt_and_load()
                .map_err(|e| format!("Failed to load credentials: {}", e))
        }
    }
}

/// Test connection to OPNsense API
#[command]
pub async fn test_api_connection(
    endpoint_url: String,
    api_key: String,
    api_secret: String,
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
        endpoint_url,
        api_key,
        api_secret,
        profile_name: None,
    };

    test_connection(&credentials)
        .await
        .map_err(|e| {
            // Format user-friendly error messages
            if e.to_string().contains("Authentication failed") {
                "Authentication failed: Invalid API key or secret. Verify credentials in OPNsense.".to_string()
            } else if e.to_string().contains("Connection refused") {
                format!("Connection refused: Unable to reach {}. Check firewall and network settings.", credentials.endpoint_url)
            } else if e.to_string().contains("certificate") {
                format!("TLS certificate error: {}. Enable 'Accept Invalid Certificates' in Advanced settings (not recommended).", e)
            } else if e.to_string().contains("timeout") {
                "Request timeout: OPNsense API did not respond within 10 seconds.".to_string()
            } else {
                format!("Connection failed: {}", e)
            }
        })
}
```

**File: src-tauri/src/main.rs (MODIFY - Register commands)**

```rust
mod api_client;
mod credentials;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Existing commands...

            // NEW: Story 3.1 API connection commands
            api_client::commands::save_api_credentials,
            api_client::commands::load_api_credentials,
            api_client::commands::test_api_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

#### **Frontend React Components**

**File: src/components/settings/api-config-settings.tsx (NEW)**

```typescript
import { useState, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { invoke } from '@tauri-apps/api/core';
import { Wifi, WifiOff, Loader2, Eye, EyeOff, CheckCircle, XCircle } from 'lucide-react';
import toast from 'react-hot-toast';

interface ApiCredentialsForm {
  endpointUrl: string;
  apiKey: string;
  apiSecret: string;
}

interface ConnectionTestResult {
  success: boolean;
  interface_count: number;
  opnsense_version?: string;
  error_message?: string;
}

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

export function ApiConfigSettings() {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected');
  const [showApiKey, setShowApiKey] = useState(false);
  const [showApiSecret, setShowApiSecret] = useState(false);
  const [testResult, setTestResult] = useState<ConnectionTestResult | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const {
    register,
    handleSubmit,
    setValue,
    watch,
    formState: { errors, isValid },
  } = useForm<ApiCredentialsForm>({
    mode: 'onChange',
  });

  const endpointUrl = watch('endpointUrl');

  // Load credentials on mount
  useEffect(() => {
    loadCredentials();
  }, []);

  const loadCredentials = async () => {
    try {
      const credentials = await invoke<ApiCredentialsForm | null>('load_api_credentials');
      if (credentials) {
        setValue('endpointUrl', credentials.endpointUrl);
        setValue('apiKey', credentials.apiKey);
        setValue('apiSecret', credentials.apiSecret);

        // Auto-connect in background
        setConnectionStatus('connecting');
        testConnectionSilent(credentials);
      }
    } catch (error) {
      console.error('Failed to load credentials:', error);
    }
  };

  const testConnectionSilent = async (credentials: ApiCredentialsForm) => {
    try {
      const result = await invoke<ConnectionTestResult>('test_api_connection', {
        endpointUrl: credentials.endpointUrl,
        apiKey: credentials.apiKey,
        apiSecret: credentials.apiSecret,
      });

      if (result.success) {
        setConnectionStatus('connected');
        setTestResult(result);
      } else {
        setConnectionStatus('error');
      }
    } catch (error) {
      setConnectionStatus('error');
    }
  };

  const onTestConnection = async (data: ApiCredentialsForm) => {
    setIsTesting(true);
    setConnectionStatus('connecting');

    try {
      const result = await invoke<ConnectionTestResult>('test_api_connection', {
        endpointUrl: data.endpointUrl,
        apiKey: data.apiKey,
        apiSecret: data.apiSecret,
      });

      if (result.success) {
        setConnectionStatus('connected');
        setTestResult(result);
        toast.success(
          `Connected to OPNsense${result.opnsense_version ? ` v${result.opnsense_version}` : ''}. Found ${result.interface_count} interfaces.`
        );
      } else {
        setConnectionStatus('error');
        toast.error(result.error_message || 'Connection test failed');
      }
    } catch (error) {
      setConnectionStatus('error');
      toast.error(String(error));
    } finally {
      setIsTesting(false);
    }
  };

  const onSave = async (data: ApiCredentialsForm) => {
    setIsSaving(true);

    try {
      await invoke('save_api_credentials', {
        endpointUrl: data.endpointUrl,
        apiKey: data.apiKey,
        apiSecret: data.apiSecret,
      });

      toast.success('Credentials saved securely');
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="max-w-2xl mx-auto p-6 bg-white dark:bg-gray-900 rounded-lg shadow">
      <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-6">
        OPNsense API Configuration
      </h2>

      {/* Connection Status Indicator */}
      <div className="mb-6 flex items-center gap-3">
        <ConnectionStatusBadge status={connectionStatus} result={testResult} />
      </div>

      <form className="space-y-4">
        {/* Endpoint URL */}
        <div>
          <label htmlFor="endpointUrl" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Endpoint URL
          </label>
          <input
            id="endpointUrl"
            type="text"
            placeholder="https://192.168.1.1"
            {...register('endpointUrl', {
              required: 'Endpoint URL is required',
              pattern: {
                value: /^https:\/\/.+/,
                message: 'Must start with https://',
              },
            })}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700
              bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
              rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          {errors.endpointUrl && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">{errors.endpointUrl.message}</p>
          )}
        </div>

        {/* API Key */}
        <div>
          <label htmlFor="apiKey" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            API Key
          </label>
          <div className="relative">
            <input
              id="apiKey"
              type={showApiKey ? 'text' : 'password'}
              {...register('apiKey', { required: 'API Key is required' })}
              className="w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-700
                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
                rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            <button
              type="button"
              onClick={() => setShowApiKey(!showApiKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1
                text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
            >
              {showApiKey ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
            </button>
          </div>
          {errors.apiKey && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">{errors.apiKey.message}</p>
          )}
        </div>

        {/* API Secret */}
        <div>
          <label htmlFor="apiSecret" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            API Secret
          </label>
          <div className="relative">
            <input
              id="apiSecret"
              type={showApiSecret ? 'text' : 'password'}
              {...register('apiSecret', { required: 'API Secret is required' })}
              className="w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-700
                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
                rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            <button
              type="button"
              onClick={() => setShowApiSecret(!showApiSecret)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1
                text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
            >
              {showApiSecret ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
            </button>
          </div>
          {errors.apiSecret && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">{errors.apiSecret.message}</p>
          )}
        </div>

        {/* Action Buttons */}
        <div className="flex gap-3 pt-4">
          <button
            type="button"
            onClick={handleSubmit(onTestConnection)}
            disabled={!isValid || isTesting}
            className="px-4 py-2 bg-blue-600 text-white rounded
              hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed
              flex items-center gap-2 transition-colors"
          >
            {isTesting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Testing...
              </>
            ) : (
              <>
                <Wifi className="w-4 h-4" />
                Test Connection
              </>
            )}
          </button>

          <button
            type="button"
            onClick={handleSubmit(onSave)}
            disabled={!isValid || isSaving}
            className="px-4 py-2 bg-green-600 text-white rounded
              hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed
              transition-colors"
          >
            {isSaving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </form>
    </div>
  );
}

function ConnectionStatusBadge({
  status,
  result,
}: {
  status: ConnectionStatus;
  result: ConnectionTestResult | null;
}) {
  const statusConfig = {
    disconnected: {
      icon: WifiOff,
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-100 dark:bg-red-900/30',
      label: 'Disconnected',
    },
    connecting: {
      icon: Loader2,
      color: 'text-yellow-600 dark:text-yellow-400',
      bgColor: 'bg-yellow-100 dark:bg-yellow-900/30',
      label: 'Connecting...',
      animated: true,
    },
    connected: {
      icon: CheckCircle,
      color: 'text-green-600 dark:text-green-400',
      bgColor: 'bg-green-100 dark:bg-green-900/30',
      label: result?.opnsense_version ? `Connected to OPNsense v${result.opnsense_version}` : 'Connected',
    },
    error: {
      icon: XCircle,
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-100 dark:bg-red-900/30',
      label: 'Connection Failed',
    },
  };

  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <div className={`inline-flex items-center gap-2 px-3 py-2 rounded-lg ${config.bgColor}`}>
      <Icon className={`w-5 h-5 ${config.color} ${config.animated ? 'animate-spin' : ''}`} />
      <span className={`text-sm font-medium ${config.color}`}>{config.label}</span>
    </div>
  );
}
```

---

### Previous Story Intelligence (Stories 2.1-2.4 Learnings)

**From Epic 2 (Filter & Search):**
- ✅ Zustand persist pattern - Use for API connection state if needed
- ✅ React Hook Form pattern - REUSE for API config form
- ✅ Tailwind styling - FOLLOW established dark/light theme patterns
- ✅ Toast notifications - REUSE for success/error feedback
- ✅ Icon usage - lucide-react icons established (Wifi, Eye, Settings)

**From Story 0.2 (Test Infrastructure):**
- ✅ cargo test framework - Use for backend unit tests
- ✅ Vitest + React Testing Library - Use for frontend component tests
- ✅ 80%+ frontend coverage, 85%+ backend coverage targets

**Key Implementation Patterns:**
1. **Security First**: Credentials NEVER in logs, error messages, or debug output
2. **Graceful Fallback**: Keychain → Encrypted file → Clear error message
3. **HTTPS Enforcement**: Reject HTTP URLs at frontend AND backend
4. **User-Friendly Errors**: Specific actionable messages per AC
5. **Async Non-Blocking**: Auto-connect on startup happens in background

---

### Git Intelligence Summary

**Recent Commit Patterns:**
- ✅ `e325b36` - Story 2.4 search history implementation
- ✅ `6a8f7a4` - Story 2.3 filter management
- ✅ Pattern: `feat: [description] (Story X.Y)` for new features
- ✅ Pattern: `fix: [description] (Story X.Y)` for bug fixes

**Commit Format for Story 3.1:**
```
feat: implement API connection setup with credential storage (Story 3.1)

- Add keyring 3.6.3 for OS keychain integration (Windows/macOS/Linux)
- Add reqwest 0.13.1 with retry middleware for OPNsense API client
- Implement AES-256-GCM encrypted fallback storage with Argon2id
- Create ApiConfigSettings component with connection testing
- Enforce HTTPS-only connections per NFR-003.4
- Auto-load credentials on app startup with background connection test
- Add comprehensive unit tests (85%+ backend, 80%+ frontend coverage)
```

---

### Testing Strategy

**Backend Unit Tests (85%+ coverage required):**

**credentials/manager.rs:**
- Test save_credentials to keyring
- Test load_credentials from keyring
- Test delete_credentials
- Test keyring failure scenarios
- Test serialization/deserialization
- Mock keyring::Entry for deterministic tests

**credentials/encrypted_storage.rs:**
- Test encrypt_and_save with AES-256-GCM
- Test decrypt_and_load
- Test Argon2id key derivation stability
- Test device seed generation (stable across runs)
- Test file corruption handling
- Test decryption with wrong key (should fail gracefully)

**api_client/client.rs:**
- Test build_api_client with HTTPS enforcement
- Test build_api_client rejects HTTP URLs
- Test retry logic with mock server (3 retries, exponential backoff)
- Test timeout after 10 seconds
- Test API key/secret header injection
- Test test_connection success path
- Test test_connection with auth failure (401)
- Test test_connection with network error
- Test test_connection with TLS error
- Test test_connection with timeout

**api_client/commands.rs:**
- Test save_api_credentials validation (empty fields, HTTP rejection)
- Test save_api_credentials with keyring success
- Test save_api_credentials with keyring failure → encrypted fallback
- Test load_api_credentials from keyring
- Test load_api_credentials from encrypted fallback
- Test test_api_connection validation
- Test test_api_connection error message formatting

**Frontend Unit Tests (80%+ coverage required):**

**api-config-settings.tsx:**
- Test rendering with empty form
- Test rendering with loaded credentials
- Test HTTPS validation (reject HTTP URLs)
- Test non-empty validation
- Test show/hide password toggle
- Test Test Connection button behavior (loading, success, error)
- Test Save button behavior (loading, success, error)
- Test connection status indicator updates (disconnected, connecting, connected, error)
- Test auto-load on mount
- Test toast notifications (success, error)

**Integration Tests:**
- End-to-end: Enter credentials → Test Connection → Success → Save
- End-to-end: Save credentials → Restart app → Auto-load → Auto-connect
- End-to-end: Invalid credentials → Test Connection → Auth error with specific message
- End-to-end: Keyring unavailable → Fallback to encrypted storage → Success
- End-to-end: HTTP URL → Rejected at frontend validation
- End-to-end: HTTP URL → Rejected at backend validation
- End-to-end: Connection timeout → Error message with timeout details

**Security Audit:**
- ✅ Verify credentials NEVER appear in application logs
- ✅ Verify credentials NEVER appear in error messages (even in debug mode)
- ✅ Verify HTTPS enforcement (HTTP rejected at frontend AND backend)
- ✅ Verify AES-256-GCM encryption correctness (ring crate)
- ✅ Verify Argon2id key derivation parameters (secure defaults)
- ✅ Run cargo audit for known vulnerabilities
- ✅ Test with Tauri devtools closed (no credential exposure)

**Performance Tests:**
- Test connection: ≤10 seconds (timeout enforced)
- Save credentials: <500ms
- Load credentials: <200ms
- Auto-connect on startup: <3 seconds (background, non-blocking UI)
- Verify no memory leaks with repeated save/load cycles

---

### Critical Implementation Details

**1. Security Non-Negotiables:**
- ❌ NEVER log credentials (not even in debug/trace logs)
- ❌ NEVER include credentials in error messages
- ❌ NEVER allow HTTP endpoints (HTTPS only)
- ✅ ALWAYS use OS keychain first (Windows/macOS/Linux)
- ✅ ALWAYS fallback to AES-256-GCM encrypted file if keychain fails
- ✅ ALWAYS validate HTTPS at frontend AND backend (defense in depth)

**2. Keyring Integration:**
- Service name: "opnsense-log-viewer"
- Account name: "api-credentials"
- MVP: Single profile only (multi-profile = future story)
- Serialize credentials as JSON with serde
- Handle keyring::Error::NoEntry gracefully (first launch, no credentials)

**3. Encrypted Fallback Storage:**
- Use AES-256-GCM via ring crate (production-ready, audited)
- Use Argon2id for key derivation (OWASP recommended)
- Device seed from machine-uid crate (stable across runs, machine-specific)
- File location: app data dir / .credentials.enc
- Prepend nonce to ciphertext (12 bytes AES-GCM nonce + ciphertext)
- Graceful corruption handling (prompt user for re-entry, don't crash)

**4. HTTP Client Configuration:**
- reqwest 0.13.1 with rustls-tls (no OpenSSL dependency)
- reqwest-middleware + reqwest-retry for exponential backoff
- Retry policy: 100ms → 1600ms, max 3 retries (per architecture)
- Timeout: 10 seconds per request
- Retry only on 5xx server errors and network transient errors
- No retry on 4xx client errors (immediate fail)

**5. Test Connection Behavior:**
- Call GET /api/diagnostics/interface/getInterfaceNames (primary test)
- Optional: Call GET /api/core/firmware/status for version detection
- Parse response to count interfaces
- Return ConnectionTestResult with interface_count and opnsense_version
- Format errors with user-friendly messages per AC:
  - 401 → "Authentication failed: Invalid API key or secret..."
  - Connection refused → "Unable to reach [URL]. Check firewall..."
  - TLS error → "TLS certificate invalid: [details]..."
  - Timeout → "Request timeout after 10 seconds"

**6. Auto-Load on Startup:**
- Call load_api_credentials() on App.tsx mount (useEffect)
- If credentials exist, populate form fields (passwords hidden by default)
- Automatically attempt connection in background (silent)
- Update connection status indicator based on result
- Don't block UI rendering (async background operation)
- Silent failure if no credentials (first launch, no toast)

**7. Frontend Validation:**
- React Hook Form with mode: 'onChange' (live validation)
- HTTPS pattern: `/^https:\/\/.+/`
- Required validation for all fields (endpoint, apiKey, apiSecret)
- Disable buttons until form is valid
- Show validation errors below inputs (red text)

**8. Connection Status Indicator:**
- Disconnected: Red dot + WifiOff icon + "Disconnected"
- Connecting: Yellow dot + Loader2 icon (spinning) + "Connecting..."
- Connected: Green dot + CheckCircle icon + "Connected to OPNsense [version]"
- Error: Red dot + XCircle icon + "Connection Failed"
- Use Tailwind classes for color-coded backgrounds (subtle, not loud)

---

### Architecture Requirements Summary

**From Architecture Document:**

**Technology Stack:**
- ✅ keyring 3.6.3 for OS keychain integration
- ✅ reqwest 0.13.1 with rustls-tls for HTTP client
- ✅ reqwest-middleware + reqwest-retry for retry logic
- ✅ ring or rust-crypto for AES-256-GCM encryption
- ✅ argon2 for Argon2id key derivation
- ✅ React Hook Form 7.x for form validation
- ✅ lucide-react for icons
- ✅ Tailwind CSS for styling

**Code Organization:**
- ✅ Backend: src-tauri/src/api_client/ for API client
- ✅ Backend: src-tauri/src/credentials/ for credential management
- ✅ Frontend: src/components/settings/ for settings UI
- ✅ Frontend: src/types/ for TypeScript types
- ✅ Frontend: src/utils/ for API invoke wrappers

**Security Requirements (NFR-003):**
- ✅ NFR-003.1: Credentials encrypted at rest (OS keychain or AES-256-GCM)
- ✅ NFR-003.2: 100% local processing (API calls only to user's OPNsense)
- ✅ NFR-003.3: Input validation (HTTPS enforcement, non-empty fields)
- ✅ NFR-003.4: Secure defaults (HTTPS enforced, no HTTP allowed)
- ✅ Never log credentials in plaintext
- ✅ TLS certificate validation enabled (rustls-tls)

**Usability Requirements (NFR-004):**
- ✅ NFR-004.3: Actionable error messages with specific failure reasons
- ✅ Clear recovery steps (e.g., "Check firewall and network settings")
- ✅ Auto-load credentials on startup for seamless experience
- ✅ Show/hide password toggles for usability vs security balance

**Reliability Requirements (NFR-002):**
- ✅ NFR-002.5: Graceful degradation (keychain unavailable → encrypted fallback)
- ✅ Never crash on keyring failure, file corruption, or API errors
- ✅ Clear visual indicators for all connection states

---

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)

### Debug Log References

N/A - Story created via create-story workflow (2026-01-18)

### Completion Notes List

(To be filled during implementation)

### File List

**Backend Files to Create:**
- src-tauri/src/api_client/mod.rs
- src-tauri/src/api_client/client.rs
- src-tauri/src/api_client/types.rs
- src-tauri/src/api_client/commands.rs
- src-tauri/src/api_client/client.test.rs
- src-tauri/src/credentials/mod.rs
- src-tauri/src/credentials/manager.rs
- src-tauri/src/credentials/encrypted_storage.rs
- src-tauri/src/credentials/manager.test.rs
- src-tauri/src/credentials/encrypted_storage.test.rs

**Backend Files to Modify:**
- src-tauri/src/main.rs (register Tauri commands)
- src-tauri/Cargo.toml (add dependencies)

**Frontend Files to Create:**
- src/components/settings/api-config-settings.tsx
- src/components/settings/connection-status-badge.tsx
- src/components/settings/api-config-settings.test.tsx
- src/components/settings/index.ts
- src/components/settings-dialog.tsx
- src/types/api.ts
- src/utils/api-client.ts
- src/stores/api-connection-store.ts (optional)

**Frontend Files to Modify:**
- src/App.tsx (add auto-load credentials on mount)
- src/components/layout/toolbar.tsx (add Settings button)

---

### Story Completion Status

**Status:** ready-for-dev

**Next Steps:**
1. Add dependencies to src-tauri/Cargo.toml (keyring, reqwest, ring, argon2)
2. Create backend type definitions (ApiCredentials, ConnectionStatus, ApiError)
3. Implement OS keychain integration (credentials/manager.rs)
4. Implement AES-256-GCM encrypted fallback (credentials/encrypted_storage.rs)
5. Create OPNsense API client with retry logic (api_client/client.rs)
6. Implement test connection endpoint
7. Create Tauri commands for IPC (save, load, test)
8. Register commands in main.rs
9. Create ApiConfigSettings React component with React Hook Form
10. Implement connection status indicator
11. Implement Test Connection and Save functionality
12. Add auto-load on app startup (App.tsx useEffect)
13. Integrate Settings panel into main app (Settings dialog/page)
14. Write comprehensive backend unit tests (85%+ coverage)
15. Write comprehensive frontend unit tests (80%+ coverage)
16. Write integration tests (end-to-end workflows)
17. Security audit (cargo audit, credential leak verification)
18. Performance testing (connection timeout, save/load speed)
19. Commit: `feat: implement API connection setup with credential storage (Story 3.1)`

**Blocking Dependencies:**
- Story 0.1 (Project Scaffolding) ✅ DONE
- Story 0.2 (Test Infrastructure) ✅ DONE
- Story 0.3 (Tailwind CSS Design System) ✅ DONE

**Blocked Stories:**
- Story 3.2 (Interface Name Mapping) - Requires API client from 3.1
- Story 3.3 (Rule Label Enrichment) - Requires API client from 3.1
- Story 3.4 (Alias Resolution) - Requires API client from 3.1
- Story 3.5 (Graceful Degradation) - Requires connection status from 3.1

---
