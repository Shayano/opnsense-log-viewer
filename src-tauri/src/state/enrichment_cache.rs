use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use log::debug;

use crate::api_client::types::{AliasCache, AliasMapping, ConnectionInfo, ConnectionStatus, InterfaceMappingCache, RuleLabelCache};

/// Thread-safe in-memory cache for interface mappings, rule labels, and IP aliases
#[derive(Clone)]
pub struct EnrichmentCacheState {
    interface_cache: Arc<Mutex<Option<InterfaceMappingCache>>>,
    rule_label_cache: Arc<Mutex<HashMap<String, String>>>,
    rule_label_metadata: Arc<Mutex<Option<DateTime<Utc>>>>,
    alias_cache: Arc<Mutex<HashMap<String, Vec<AliasMapping>>>>,
    alias_metadata: Arc<Mutex<Option<DateTime<Utc>>>>,
    device_id: Arc<Mutex<Option<String>>>,
    // Connection status tracking (Story 3.5)
    connection_status: Arc<Mutex<ConnectionStatus>>,
    last_error: Arc<Mutex<Option<String>>>,
    last_api_check: Arc<Mutex<DateTime<Utc>>>,
}

impl EnrichmentCacheState {
    pub fn new() -> Self {
        Self {
            interface_cache: Arc::new(Mutex::new(None)),
            rule_label_cache: Arc::new(Mutex::new(HashMap::new())),
            rule_label_metadata: Arc::new(Mutex::new(None)),
            alias_cache: Arc::new(Mutex::new(HashMap::new())),
            alias_metadata: Arc::new(Mutex::new(None)),
            device_id: Arc::new(Mutex::new(None)),
            // Initialize connection status as Disconnected
            connection_status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            last_error: Arc::new(Mutex::new(None)),
            last_api_check: Arc::new(Mutex::new(Utc::now())),
        }
    }

    /// Store interface mappings in cache
    pub fn set_interface_mappings(
        &self,
        mappings: HashMap<String, String>,
        device_id: String,
    ) {
        let cache = InterfaceMappingCache {
            mappings,
            last_updated: Utc::now(),
            device_id,
        };

        let mut cache_guard = self.interface_cache.lock().unwrap();
        *cache_guard = Some(cache);

        debug!("Interface mappings cached");
    }

    /// Get logical name for a physical interface
    pub fn get_interface_mapping(&self, physical_name: &str) -> Option<String> {
        let cache_guard = self.interface_cache.lock().unwrap();

        cache_guard
            .as_ref()
            .and_then(|cache| cache.mappings.get(physical_name).cloned())
    }

    /// Get all interface mappings
    pub fn get_all_interface_mappings(&self) -> Option<InterfaceMappingCache> {
        let cache_guard = self.interface_cache.lock().unwrap();
        cache_guard.clone()
    }

    /// Clear interface cache (e.g., when switching OPNsense devices)
    pub fn clear_interface_mappings(&self) {
        let mut cache_guard = self.interface_cache.lock().unwrap();
        *cache_guard = None;

        debug!("Interface mappings cache cleared");
    }

    // ============================================================================
    // Rule Label Cache Methods (Story 3.3)
    // ============================================================================

    /// Store rule label in cache
    pub fn set_rule_label(&self, hash: String, description: String) {
        let mut cache = self.rule_label_cache.lock().unwrap();
        cache.insert(hash, description);

        // Update metadata timestamp
        let mut metadata = self.rule_label_metadata.lock().unwrap();
        *metadata = Some(Utc::now());
    }

    /// Store multiple rule labels (batch)
    pub fn set_rule_labels(&self, labels: HashMap<String, String>, device_id: String) {
        let mut cache = self.rule_label_cache.lock().unwrap();
        cache.extend(labels);

        // Update metadata
        let mut metadata = self.rule_label_metadata.lock().unwrap();
        *metadata = Some(Utc::now());

        let mut device = self.device_id.lock().unwrap();
        *device = Some(device_id);

        debug!("Rule labels cached: {} entries", cache.len());
    }

    /// Get rule label for a specific hash
    pub fn get_rule_label(&self, hash: &str) -> Option<String> {
        let cache = self.rule_label_cache.lock().unwrap();
        cache.get(hash).cloned()
    }

    /// Get all rule labels with metadata
    pub fn get_all_rule_labels(&self) -> Option<RuleLabelCache> {
        let cache = self.rule_label_cache.lock().unwrap();
        let metadata = self.rule_label_metadata.lock().unwrap();
        let device_id = self.device_id.lock().unwrap();

        if cache.is_empty() {
            return None;
        }

        Some(RuleLabelCache {
            mappings: cache.clone(),
            last_updated: metadata.unwrap_or(Utc::now()),
            device_id: device_id.clone().unwrap_or_default(),
        })
    }

    /// Clear rule label cache (e.g., when switching devices)
    pub fn clear_rule_labels(&self) {
        let mut cache = self.rule_label_cache.lock().unwrap();
        cache.clear();

        let mut metadata = self.rule_label_metadata.lock().unwrap();
        *metadata = None;

        debug!("Rule label cache cleared");
    }

    // ============================================================================
    // Alias Cache Methods (Story 3.4)
    // ============================================================================

    /// Store alias mapping for single IP
    pub fn set_alias(&self, ip: String, aliases: Vec<AliasMapping>) {
        let mut cache = self.alias_cache.lock().unwrap();
        cache.insert(ip, aliases);

        // Update metadata timestamp
        let mut metadata = self.alias_metadata.lock().unwrap();
        *metadata = Some(Utc::now());
    }

    /// Store multiple alias mappings (batch)
    pub fn set_aliases(&self, alias_map: HashMap<String, Vec<AliasMapping>>, device_id: String) {
        let mut cache = self.alias_cache.lock().unwrap();
        cache.extend(alias_map);

        // Update metadata
        let mut metadata = self.alias_metadata.lock().unwrap();
        *metadata = Some(Utc::now());

        let mut device = self.device_id.lock().unwrap();
        *device = Some(device_id);

        debug!("Aliases cached: {} IPs", cache.len());
    }

    /// Get aliases for a specific IP
    pub fn get_alias(&self, ip: &str) -> Option<Vec<AliasMapping>> {
        let cache = self.alias_cache.lock().unwrap();
        cache.get(ip).cloned()
    }

    /// Get all aliases with metadata
    pub fn get_all_aliases(&self) -> Option<AliasCache> {
        let cache = self.alias_cache.lock().unwrap();
        let metadata = self.alias_metadata.lock().unwrap();
        let device_id = self.device_id.lock().unwrap();

        if cache.is_empty() {
            return None;
        }

        Some(AliasCache {
            mappings: cache.clone(),
            last_updated: metadata.unwrap_or(Utc::now()),
            device_id: device_id.clone().unwrap_or_default(),
        })
    }

    /// Clear alias cache (e.g., when switching devices)
    pub fn clear_aliases(&self) {
        let mut cache = self.alias_cache.lock().unwrap();
        cache.clear();

        let mut metadata = self.alias_metadata.lock().unwrap();
        *metadata = None;

        debug!("Alias cache cleared");
    }

    // ============================================================================
    // Connection Status Methods (Story 3.5)
    // ============================================================================

    /// Update connection status
    pub fn set_connection_status(&self, status: ConnectionStatus, error: Option<String>) {
        let mut status_lock = self.connection_status.lock().unwrap();
        *status_lock = status;

        let mut error_lock = self.last_error.lock().unwrap();
        *error_lock = error.clone();

        let mut check_lock = self.last_api_check.lock().unwrap();
        *check_lock = Utc::now();

        debug!("Connection status updated to {:?}, error: {:?}", status, error);
    }

    /// Get current connection status with metadata
    pub fn get_connection_info(&self) -> ConnectionInfo {
        let status = *self.connection_status.lock().unwrap();
        let last_error = self.last_error.lock().unwrap().clone();
        let last_checked = *self.last_api_check.lock().unwrap();

        ConnectionInfo {
            status,
            last_error,
            last_checked,
        }
    }

    /// Check if API is currently connected
    pub fn is_connected(&self) -> bool {
        *self.connection_status.lock().unwrap() == ConnectionStatus::Connected
    }
}

impl Default for EnrichmentCacheState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_interface_mapping() {
        let cache = EnrichmentCacheState::new();

        let mut mappings = HashMap::new();
        mappings.insert("vtnet0".to_string(), "LAN".to_string());
        mappings.insert("vtnet1".to_string(), "WAN".to_string());

        cache.set_interface_mappings(mappings, "https://192.168.1.1".to_string());

        // Test single mapping retrieval
        assert_eq!(
            cache.get_interface_mapping("vtnet0"),
            Some("LAN".to_string())
        );
        assert_eq!(
            cache.get_interface_mapping("vtnet1"),
            Some("WAN".to_string())
        );
        assert_eq!(cache.get_interface_mapping("vtnet2"), None);

        // Test full cache retrieval
        let full_cache = cache.get_all_interface_mappings().unwrap();
        assert_eq!(full_cache.mappings.len(), 2);
        assert_eq!(full_cache.device_id, "https://192.168.1.1");
    }

    #[test]
    fn test_clear_interface_mappings() {
        let cache = EnrichmentCacheState::new();

        let mut mappings = HashMap::new();
        mappings.insert("vtnet0".to_string(), "LAN".to_string());

        cache.set_interface_mappings(mappings, "device1".to_string());
        assert!(cache.get_all_interface_mappings().is_some());

        cache.clear_interface_mappings();
        assert!(cache.get_all_interface_mappings().is_none());
    }

    #[test]
    fn test_empty_cache() {
        let cache = EnrichmentCacheState::new();

        assert_eq!(cache.get_interface_mapping("vtnet0"), None);
        assert!(cache.get_all_interface_mappings().is_none());
    }

    #[test]
    fn test_cache_update() {
        let cache = EnrichmentCacheState::new();

        // First set
        let mut mappings1 = HashMap::new();
        mappings1.insert("vtnet0".to_string(), "LAN".to_string());
        cache.set_interface_mappings(mappings1, "device1".to_string());

        assert_eq!(
            cache.get_interface_mapping("vtnet0"),
            Some("LAN".to_string())
        );

        // Update with new mappings
        let mut mappings2 = HashMap::new();
        mappings2.insert("vtnet0".to_string(), "WAN".to_string());
        mappings2.insert("vtnet1".to_string(), "DMZ".to_string());
        cache.set_interface_mappings(mappings2, "device2".to_string());

        assert_eq!(
            cache.get_interface_mapping("vtnet0"),
            Some("WAN".to_string())
        );
        assert_eq!(
            cache.get_interface_mapping("vtnet1"),
            Some("DMZ".to_string())
        );

        let full_cache = cache.get_all_interface_mappings().unwrap();
        assert_eq!(full_cache.mappings.len(), 2);
        assert_eq!(full_cache.device_id, "device2");
    }

    // ============================================================================
    // Rule Label Cache Tests (Story 3.3)
    // ============================================================================

    #[test]
    fn test_set_and_get_rule_label() {
        let cache = EnrichmentCacheState::new();

        cache.set_rule_label("abc123".to_string(), "Block RFC1918".to_string());
        cache.set_rule_label("def456".to_string(), "Allow HTTPS".to_string());

        assert_eq!(cache.get_rule_label("abc123"), Some("Block RFC1918".to_string()));
        assert_eq!(cache.get_rule_label("def456"), Some("Allow HTTPS".to_string()));
        assert_eq!(cache.get_rule_label("xyz789"), None);
    }

    #[test]
    fn test_set_rule_labels_batch() {
        let cache = EnrichmentCacheState::new();

        let mut labels = HashMap::new();
        labels.insert("hash1".to_string(), "Rule 1".to_string());
        labels.insert("hash2".to_string(), "Rule 2".to_string());

        cache.set_rule_labels(labels, "device1".to_string());

        let all_labels = cache.get_all_rule_labels().unwrap();
        assert_eq!(all_labels.mappings.len(), 2);
        assert_eq!(all_labels.device_id, "device1");
    }

    #[test]
    fn test_clear_rule_labels() {
        let cache = EnrichmentCacheState::new();
        cache.set_rule_label("hash".to_string(), "Label".to_string());
        assert!(cache.get_all_rule_labels().is_some());

        cache.clear_rule_labels();
        assert!(cache.get_all_rule_labels().is_none());
    }

    #[test]
    fn test_empty_rule_label_cache() {
        let cache = EnrichmentCacheState::new();

        assert_eq!(cache.get_rule_label("abc123"), None);
        assert!(cache.get_all_rule_labels().is_none());
    }

    #[test]
    fn test_rule_label_cache_update() {
        let cache = EnrichmentCacheState::new();

        // First batch
        let mut labels1 = HashMap::new();
        labels1.insert("hash1".to_string(), "Label 1".to_string());
        cache.set_rule_labels(labels1, "device1".to_string());

        assert_eq!(cache.get_rule_label("hash1"), Some("Label 1".to_string()));

        // Second batch extends cache
        let mut labels2 = HashMap::new();
        labels2.insert("hash2".to_string(), "Label 2".to_string());
        labels2.insert("hash3".to_string(), "Label 3".to_string());
        cache.set_rule_labels(labels2, "device2".to_string());

        // All labels should be present
        assert_eq!(cache.get_rule_label("hash1"), Some("Label 1".to_string()));
        assert_eq!(cache.get_rule_label("hash2"), Some("Label 2".to_string()));
        assert_eq!(cache.get_rule_label("hash3"), Some("Label 3".to_string()));

        let all_labels = cache.get_all_rule_labels().unwrap();
        assert_eq!(all_labels.mappings.len(), 3);
        assert_eq!(all_labels.device_id, "device2");
    }

    // ============================================================================
    // Connection Status Tests (Story 3.5)
    // ============================================================================

    #[test]
    fn test_connection_status_tracking() {
        let cache = EnrichmentCacheState::new();

        // Initial status is disconnected
        assert_eq!(cache.get_connection_info().status, ConnectionStatus::Disconnected);
        assert!(!cache.is_connected());

        // Update to connected
        cache.set_connection_status(ConnectionStatus::Connected, None);
        assert_eq!(cache.get_connection_info().status, ConnectionStatus::Connected);
        assert!(cache.is_connected());

        // Update to disconnected with error
        cache.set_connection_status(
            ConnectionStatus::Disconnected,
            Some("Network error".to_string())
        );
        let info = cache.get_connection_info();
        assert_eq!(info.status, ConnectionStatus::Disconnected);
        assert_eq!(info.last_error.unwrap(), "Network error");
        assert!(!cache.is_connected());
    }

    #[test]
    fn test_connection_status_transitions() {
        let cache = EnrichmentCacheState::new();

        // Disconnected → Connected
        cache.set_connection_status(ConnectionStatus::Connected, None);
        assert_eq!(cache.get_connection_info().status, ConnectionStatus::Connected);

        // Connected → Degraded
        cache.set_connection_status(
            ConnectionStatus::Degraded,
            Some("Some calls timing out".to_string())
        );
        assert_eq!(cache.get_connection_info().status, ConnectionStatus::Degraded);
        assert!(!cache.is_connected()); // Only Connected returns true

        // Degraded → Connected
        cache.set_connection_status(ConnectionStatus::Connected, None);
        assert_eq!(cache.get_connection_info().status, ConnectionStatus::Connected);
        assert!(cache.is_connected());
    }

    #[test]
    fn test_connection_info_metadata() {
        let cache = EnrichmentCacheState::new();

        cache.set_connection_status(
            ConnectionStatus::Disconnected,
            Some("Test error".to_string())
        );

        let info = cache.get_connection_info();
        assert_eq!(info.status, ConnectionStatus::Disconnected);
        assert_eq!(info.last_error.unwrap(), "Test error");
        assert!(info.last_checked <= Utc::now());
    }

    #[test]
    fn test_connection_error_clearing() {
        let cache = EnrichmentCacheState::new();

        // Set error
        cache.set_connection_status(
            ConnectionStatus::Disconnected,
            Some("Error message".to_string())
        );
        assert!(cache.get_connection_info().last_error.is_some());

        // Clear error
        cache.set_connection_status(ConnectionStatus::Connected, None);
        assert!(cache.get_connection_info().last_error.is_none());
    }
}
