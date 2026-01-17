use roaring::RoaringBitmap;
use std::collections::HashMap;

/// Bitmap index for low-cardinality fields (action, protocol, interface)
/// Uses compressed bitmaps for efficient set operations
pub struct BitmapIndex {
    actions: HashMap<String, RoaringBitmap>,
    protocols: HashMap<String, RoaringBitmap>,
    interfaces: HashMap<String, RoaringBitmap>,
}

impl Default for BitmapIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl BitmapIndex {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            protocols: HashMap::new(),
            interfaces: HashMap::new(),
        }
    }

    /// Add an entry to the bitmap index
    pub fn add_entry(
        &mut self,
        entry_id: u64,
        action: Option<&str>,
        protocol: Option<&str>,
        interface: Option<&str>,
    ) {
        if let Some(action) = action {
            self.actions
                .entry(action.to_lowercase())
                .or_default()
                .insert(entry_id as u32); // RoaringBitmap uses u32
        }

        if let Some(protocol) = protocol {
            self.protocols
                .entry(protocol.to_uppercase())
                .or_default()
                .insert(entry_id as u32);
        }

        if let Some(interface) = interface {
            self.interfaces
                .entry(interface.to_string())
                .or_default()
                .insert(entry_id as u32);
        }
    }

    /// Query actions - returns bitmap of entry IDs matching the given action
    pub fn query_action(&self, action: &str) -> Option<&RoaringBitmap> {
        self.actions.get(&action.to_lowercase())
    }

    /// Query protocols - returns bitmap of entry IDs matching the given protocol
    pub fn query_protocol(&self, protocol: &str) -> Option<&RoaringBitmap> {
        self.protocols.get(&protocol.to_uppercase())
    }

    /// Query interfaces - returns bitmap of entry IDs matching the given interface
    pub fn query_interface(&self, interface: &str) -> Option<&RoaringBitmap> {
        self.interfaces.get(interface)
    }

    /// Perform intersection of multiple bitmaps (AND operation)
    pub fn intersect(&self, bitmaps: Vec<&RoaringBitmap>) -> RoaringBitmap {
        if bitmaps.is_empty() {
            return RoaringBitmap::new();
        }

        let mut result = bitmaps[0].clone();
        for bitmap in &bitmaps[1..] {
            result &= *bitmap;
        }
        result
    }

    /// Perform union of multiple bitmaps (OR operation)
    pub fn union(&self, bitmaps: Vec<&RoaringBitmap>) -> RoaringBitmap {
        let mut result = RoaringBitmap::new();
        for bitmap in bitmaps {
            result |= bitmap;
        }
        result
    }

    /// Perform negation of a bitmap (NOT operation)
    /// Returns all entry IDs NOT in the given bitmap, up to max_entry_id
    pub fn negate(&self, bitmap: &RoaringBitmap, max_entry_id: u32) -> RoaringBitmap {
        let all_entries = RoaringBitmap::from_sorted_iter(0..=max_entry_id).unwrap();
        all_entries - bitmap
    }

    /// Get approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let mut size = 0;

        for (key, bitmap) in &self.actions {
            size += key.len() + bitmap.serialized_size();
        }
        for (key, bitmap) in &self.protocols {
            size += key.len() + bitmap.serialized_size();
        }
        for (key, bitmap) in &self.interfaces {
            size += key.len() + bitmap.serialized_size();
        }

        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_index_insert_and_query() {
        let mut index = BitmapIndex::new();

        // Add entries
        index.add_entry(1, Some("block"), Some("TCP"), Some("vtnet0"));
        index.add_entry(2, Some("pass"), Some("UDP"), Some("vtnet1"));
        index.add_entry(3, Some("block"), Some("TCP"), Some("vtnet0"));

        // Query action
        let results = index.query_action("block").unwrap();
        assert!(results.contains(1));
        assert!(results.contains(3));
        assert!(!results.contains(2));

        // Query protocol
        let results = index.query_protocol("TCP").unwrap();
        assert!(results.contains(1));
        assert!(results.contains(3));
        assert!(!results.contains(2));

        // Query interface
        let results = index.query_interface("vtnet0").unwrap();
        assert!(results.contains(1));
        assert!(results.contains(3));
        assert!(!results.contains(2));
    }

    #[test]
    fn test_bitmap_intersection_and_union() {
        let mut index = BitmapIndex::new();

        index.add_entry(1, Some("block"), Some("TCP"), Some("vtnet0"));
        index.add_entry(2, Some("pass"), Some("UDP"), Some("vtnet1"));
        index.add_entry(3, Some("block"), Some("UDP"), Some("vtnet0"));

        let block_bitmap = index.query_action("block").unwrap();
        let tcp_bitmap = index.query_protocol("TCP").unwrap();
        let udp_bitmap = index.query_protocol("UDP").unwrap();

        // Intersection: block AND TCP = entry 1
        let result = index.intersect(vec![block_bitmap, tcp_bitmap]);
        assert_eq!(result.len(), 1);
        assert!(result.contains(1));

        // Union: TCP OR UDP = entries 1, 2, 3
        let result = index.union(vec![tcp_bitmap, udp_bitmap]);
        assert_eq!(result.len(), 3);
        assert!(result.contains(1));
        assert!(result.contains(2));
        assert!(result.contains(3));
    }

    #[test]
    fn test_bitmap_negation() {
        let mut index = BitmapIndex::new();

        index.add_entry(0, Some("block"), None, None);
        index.add_entry(1, Some("pass"), None, None);
        index.add_entry(2, Some("block"), None, None);

        let block_bitmap = index.query_action("block").unwrap();

        // NOT block = entry 1 (entries 0 and 2 have "block", entry 1 has "pass")
        let result = index.negate(block_bitmap, 2);
        assert_eq!(result.len(), 1);
        assert!(result.contains(1));
    }
}
