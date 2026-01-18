use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::api_client::types::{InterfaceMappingCache, RuleLabelCache};

/// Thread-safe in-memory cache for interface mappings and rule labels
#[derive(Clone)]
pub struct EnrichmentCacheState {
    interface_cache: Arc<Mutex<Option<InterfaceMappingCache>>>,
    rule_label_cache: Arc<Mutex<HashMap<String, String>>>,
    rule_label_metadata: Arc<Mutex<Option<DateTime<Utc>>>>,
    device_id: Arc<Mutex<Option<String>>>,
}

impl EnrichmentCacheState {
    pub fn new() -> Self {
        Self {
            interface_cache: Arc::new(Mutex::new(None)),
            rule_label_cache: Arc::new(Mutex::new(HashMap::new())),
            rule_label_metadata: Arc::new(Mutex::new(None)),
            device_id: Arc::new(Mutex::new(None)),
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

        log::debug!("Interface mappings cached");
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

        log::debug!("Interface mappings cache cleared");
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

        log::debug!("Rule labels cached: {} entries", cache.len());
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

        log::debug!("Rule label cache cleared");
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
}
