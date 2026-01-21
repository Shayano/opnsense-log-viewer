use roaring::RoaringBitmap;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};
use bytecheck::CheckBytes;

/// Story 6.2 (AC3): Serializable representation for BitmapIndex with RoaringBitmap
///
/// Since RoaringBitmap doesn't implement rkyv traits natively, we use an
/// intermediate serializable format that stores bitmaps as byte arrays.
/// This struct is used for rkyv serialization while BitmapIndex remains
/// the primary in-memory representation.
#[derive(Archive, RkyvSerialize, RkyvDeserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct BitmapIndexRkyv {
    /// Actions: Vec of (key, serialized_bitmap_bytes)
    actions: Vec<(String, Vec<u8>)>,
    /// Protocols: Vec of (key, serialized_bitmap_bytes)
    protocols: Vec<(String, Vec<u8>)>,
    /// Interfaces: Vec of (key, serialized_bitmap_bytes)
    interfaces: Vec<(String, Vec<u8>)>,
}

impl BitmapIndexRkyv {
    /// Convert from BitmapIndex to serializable format
    pub fn from_bitmap_index(index: &BitmapIndex) -> Self {
        Self {
            actions: index.actions.iter().map(|(k, v)| {
                let mut bytes = Vec::new();
                v.serialize_into(&mut bytes).expect("RoaringBitmap serialization");
                (k.clone(), bytes)
            }).collect(),
            protocols: index.protocols.iter().map(|(k, v)| {
                let mut bytes = Vec::new();
                v.serialize_into(&mut bytes).expect("RoaringBitmap serialization");
                (k.clone(), bytes)
            }).collect(),
            interfaces: index.interfaces.iter().map(|(k, v)| {
                let mut bytes = Vec::new();
                v.serialize_into(&mut bytes).expect("RoaringBitmap serialization");
                (k.clone(), bytes)
            }).collect(),
        }
    }

    /// Convert to BitmapIndex from serializable format
    pub fn to_bitmap_index(&self) -> BitmapIndex {
        let actions = self.actions.iter().map(|(k, v)| {
            let bitmap = RoaringBitmap::deserialize_from(v.as_slice())
                .expect("RoaringBitmap deserialization");
            (k.clone(), bitmap)
        }).collect();

        let protocols = self.protocols.iter().map(|(k, v)| {
            let bitmap = RoaringBitmap::deserialize_from(v.as_slice())
                .expect("RoaringBitmap deserialization");
            (k.clone(), bitmap)
        }).collect();

        let interfaces = self.interfaces.iter().map(|(k, v)| {
            let bitmap = RoaringBitmap::deserialize_from(v.as_slice())
                .expect("RoaringBitmap deserialization");
            (k.clone(), bitmap)
        }).collect();

        BitmapIndex { actions, protocols, interfaces }
    }
}

impl ArchivedBitmapIndexRkyv {
    /// Convert from archived format to BitmapIndex (zero-copy friendly access)
    pub fn to_bitmap_index(&self) -> BitmapIndex {
        let actions = self.actions.iter().map(|(k, v)| {
            let bitmap = RoaringBitmap::deserialize_from(v.as_slice())
                .expect("RoaringBitmap deserialization");
            (k.to_string(), bitmap)
        }).collect();

        let protocols = self.protocols.iter().map(|(k, v)| {
            let bitmap = RoaringBitmap::deserialize_from(v.as_slice())
                .expect("RoaringBitmap deserialization");
            (k.to_string(), bitmap)
        }).collect();

        let interfaces = self.interfaces.iter().map(|(k, v)| {
            let bitmap = RoaringBitmap::deserialize_from(v.as_slice())
                .expect("RoaringBitmap deserialization");
            (k.to_string(), bitmap)
        }).collect();

        BitmapIndex { actions, protocols, interfaces }
    }
}

/// Bitmap index for low-cardinality fields (action, protocol, interface)
/// Uses compressed bitmaps for efficient set operations
///
/// Story 6.2: Use BitmapIndexRkyv for serialization via `to_rkyv()`/`from_rkyv()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitmapIndex {
    actions: HashMap<String, RoaringBitmap>,
    protocols: HashMap<String, RoaringBitmap>,
    interfaces: HashMap<String, RoaringBitmap>,
}

impl BitmapIndex {
    /// Convert to rkyv-serializable format
    pub fn to_rkyv(&self) -> BitmapIndexRkyv {
        BitmapIndexRkyv::from_bitmap_index(self)
    }

    /// Create from rkyv-serializable format
    pub fn from_rkyv(rkyv: &BitmapIndexRkyv) -> Self {
        rkyv.to_bitmap_index()
    }

    /// Create from archived rkyv format (for zero-copy access patterns)
    pub fn from_archived_rkyv(archived: &ArchivedBitmapIndexRkyv) -> Self {
        archived.to_bitmap_index()
    }
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

    /// Create from raw HashMaps (for parallel merge)
    pub fn from_raw(
        actions: HashMap<String, RoaringBitmap>,
        protocols: HashMap<String, RoaringBitmap>,
        interfaces: HashMap<String, RoaringBitmap>,
    ) -> Self {
        Self {
            actions,
            protocols,
            interfaces,
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

    /// Get all unique action values with their counts (for debugging)
    pub fn get_action_stats(&self) -> Vec<(String, u64)> {
        self.actions
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }

    /// Get all unique protocol values with their counts (for debugging)
    pub fn get_protocol_stats(&self) -> Vec<(String, u64)> {
        self.protocols
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }

    /// Get all unique interface values with their counts (for debugging)
    pub fn get_interface_stats(&self) -> Vec<(String, u64)> {
        self.interfaces
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }

    /// Story 6.3: Merge action bitmap from another batch
    pub fn merge_action(&mut self, action: &str, bitmap: RoaringBitmap) {
        let key = action.to_lowercase();
        self.actions
            .entry(key)
            .and_modify(|b| *b |= &bitmap)
            .or_insert(bitmap);
    }

    /// Story 6.3: Merge protocol bitmap from another batch
    pub fn merge_protocol(&mut self, protocol: &str, bitmap: RoaringBitmap) {
        let key = protocol.to_uppercase();
        self.protocols
            .entry(key)
            .and_modify(|b| *b |= &bitmap)
            .or_insert(bitmap);
    }

    /// Story 6.3: Merge interface bitmap from another batch
    pub fn merge_interface(&mut self, interface: &str, bitmap: RoaringBitmap) {
        self.interfaces
            .entry(interface.to_string())
            .and_modify(|b| *b |= &bitmap)
            .or_insert(bitmap);
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

    /// Story 6.2 (AC3): rkyv round-trip test for BitmapIndex using BitmapIndexRkyv
    #[test]
    fn test_bitmap_index_rkyv_roundtrip() {
        let mut index = BitmapIndex::new();

        // Add many entries to test bitmap compression
        for i in 0..10_000u64 {
            let action = if i % 3 == 0 { "block" } else if i % 3 == 1 { "pass" } else { "reject" };
            let protocol = if i % 2 == 0 { "TCP" } else { "UDP" };
            let interface = format!("vtnet{}", i % 4);
            index.add_entry(i, Some(action), Some(protocol), Some(&interface));
        }

        // Convert to rkyv-serializable format and serialize
        let rkyv_format = index.to_rkyv();
        let bytes = rkyv::to_bytes::<_, 256>(&rkyv_format).expect("rkyv serialization failed");

        // Verify serialized data is compact (RoaringBitmap compresses well)
        assert!(bytes.len() > 100, "Expected some serialized data");
        assert!(bytes.len() < 500_000, "Serialized data should be reasonably compact");

        // Validate archived data
        let archived = rkyv::check_archived_root::<BitmapIndexRkyv>(&bytes)
            .expect("rkyv validation failed");

        // Verify archived vecs have entries
        assert!(!archived.actions.is_empty(), "Archived actions should have entries");
        assert!(!archived.protocols.is_empty(), "Archived protocols should have entries");
        assert!(!archived.interfaces.is_empty(), "Archived interfaces should have entries");

        // Convert from archived format back to BitmapIndex
        let deserialized = BitmapIndex::from_archived_rkyv(archived);

        // Verify roundtrip preserves bitmap data
        let original_block = index.query_action("block").unwrap();
        let deserialized_block = deserialized.query_action("block").unwrap();
        assert_eq!(original_block.len(), deserialized_block.len(),
            "Block bitmap length mismatch");

        // Verify specific entries
        assert!(deserialized_block.contains(0), "Entry 0 should be in block bitmap");
        assert!(deserialized_block.contains(3), "Entry 3 should be in block bitmap");
        assert!(!deserialized_block.contains(1), "Entry 1 should NOT be in block bitmap");

        // Verify protocol bitmaps
        let original_tcp = index.query_protocol("TCP").unwrap();
        let deserialized_tcp = deserialized.query_protocol("TCP").unwrap();
        assert_eq!(original_tcp.len(), deserialized_tcp.len(),
            "TCP bitmap length mismatch");

        // Verify interface bitmaps
        let original_vtnet0 = index.query_interface("vtnet0").unwrap();
        let deserialized_vtnet0 = deserialized.query_interface("vtnet0").unwrap();
        assert_eq!(original_vtnet0.len(), deserialized_vtnet0.len(),
            "vtnet0 bitmap length mismatch");

        // Verify bitmap operations still work on deserialized data
        let intersect_result = deserialized.intersect(vec![deserialized_block, deserialized_tcp]);
        assert!(intersect_result.len() > 0, "Intersection should have entries");
    }

    /// Story 6.6 (AC1): rkyv round-trip test with 100K+ entries for BitmapIndex
    /// Validates data integrity at scale for bitmap indexes
    #[test]
    fn test_bitmap_index_rkyv_roundtrip_100k() {
        let mut index = BitmapIndex::new();

        // Add 100,000 entries with diverse action/protocol/interface distribution
        for i in 0..100_000u64 {
            let action = match i % 5 {
                0 => "block",
                1 => "pass",
                2 => "reject",
                3 => "nat",
                _ => "rdr",
            };
            let protocol = match i % 4 {
                0 => "TCP",
                1 => "UDP",
                2 => "ICMP",
                _ => "GRE",
            };
            let interface = format!("vtnet{}", i % 8);
            index.add_entry(i, Some(action), Some(protocol), Some(&interface));
        }

        // Convert to rkyv-serializable format and serialize
        let rkyv_format = index.to_rkyv();
        let bytes = rkyv::to_bytes::<_, 256>(&rkyv_format).expect("rkyv serialization failed");

        // Verify serialized data (RoaringBitmap compresses 100K entries efficiently)
        assert!(bytes.len() > 1_000, "Expected significant serialized data for 100K entries");
        assert!(bytes.len() < 5_000_000, "Serialized bitmap data should be compact");

        // Validate archived data
        let archived = rkyv::check_archived_root::<BitmapIndexRkyv>(&bytes)
            .expect("rkyv validation failed");

        // Verify archived vecs have entries
        assert_eq!(archived.actions.len(), 5, "Expected 5 action types");
        assert_eq!(archived.protocols.len(), 4, "Expected 4 protocol types");
        assert_eq!(archived.interfaces.len(), 8, "Expected 8 interface types");

        // Convert from archived format back to BitmapIndex
        let deserialized = BitmapIndex::from_archived_rkyv(archived);

        // Verify roundtrip preserves bitmap data for all action types
        for action in ["block", "pass", "reject", "nat", "rdr"] {
            let original = index.query_action(action).unwrap();
            let loaded = deserialized.query_action(action).unwrap();
            assert_eq!(
                original.len(), loaded.len(),
                "Action '{}' bitmap length mismatch: {} vs {}",
                action, original.len(), loaded.len()
            );
        }

        // Verify protocol bitmaps
        for protocol in ["TCP", "UDP", "ICMP", "GRE"] {
            let original = index.query_protocol(protocol).unwrap();
            let loaded = deserialized.query_protocol(protocol).unwrap();
            assert_eq!(
                original.len(), loaded.len(),
                "Protocol '{}' bitmap length mismatch", protocol
            );
        }

        // Verify interface bitmaps
        for i in 0..8 {
            let interface = format!("vtnet{}", i);
            let original = index.query_interface(&interface).unwrap();
            let loaded = deserialized.query_interface(&interface).unwrap();
            assert_eq!(
                original.len(), loaded.len(),
                "Interface '{}' bitmap length mismatch", interface
            );
        }

        // Verify expected counts (100K entries distributed across 5 actions)
        let block_count = deserialized.query_action("block").unwrap().len();
        assert_eq!(block_count, 20_000, "Expected 20K block entries (100K / 5 actions)");

        // Verify bitmap operations work on deserialized data
        let block_bitmap = deserialized.query_action("block").unwrap();
        let tcp_bitmap = deserialized.query_protocol("TCP").unwrap();
        let intersect_result = deserialized.intersect(vec![block_bitmap, tcp_bitmap]);
        // block entries: 0, 5, 10, 15... (every 5th)
        // TCP entries: 0, 4, 8, 12... (every 4th)
        // Intersection: 0, 20, 40... (every 20th) = 5000 entries
        assert_eq!(intersect_result.len(), 5_000, "Intersection count mismatch");
    }
}
