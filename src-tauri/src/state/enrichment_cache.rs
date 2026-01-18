use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;

use crate::api_client::types::InterfaceMappingCache;

/// Thread-safe in-memory cache for interface mappings
#[derive(Clone)]
pub struct EnrichmentCacheState {
    interface_cache: Arc<Mutex<Option<InterfaceMappingCache>>>,
}

impl EnrichmentCacheState {
    pub fn new() -> Self {
        Self {
            interface_cache: Arc::new(Mutex::new(None)),
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
}
