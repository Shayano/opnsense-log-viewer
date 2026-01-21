use serde::{Deserialize, Serialize};
use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};
use bytecheck::CheckBytes;

/// Offset table for random access to raw log lines
/// Maps entry IDs to byte offsets in the source file
///
/// Story 6.2: Added rkyv derives for zero-copy serialization
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct OffsetTable {
    offsets: Vec<u64>,
}

impl Default for OffsetTable {
    fn default() -> Self {
        Self::new()
    }
}

impl OffsetTable {
    pub fn new() -> Self {
        Self {
            offsets: Vec::new(),
        }
    }

    /// Create with pre-allocated capacity (for parallel merge)
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            offsets: vec![0; capacity],
        }
    }

    /// Add an offset for an entry (grows vector if needed)
    pub fn add_offset(&mut self, entry_id: u64, offset: u64) {
        // Ensure vector is large enough
        if entry_id as usize >= self.offsets.len() {
            self.offsets.resize(entry_id as usize + 1, 0);
        }
        self.offsets[entry_id as usize] = offset;
    }

    /// Set offset directly (assumes capacity is already allocated)
    #[inline]
    pub fn set_offset(&mut self, entry_id: u64, offset: u64) {
        if (entry_id as usize) < self.offsets.len() {
            self.offsets[entry_id as usize] = offset;
        }
    }

    /// Get the offset for an entry
    pub fn get_offset(&self, entry_id: u64) -> Option<u64> {
        self.offsets.get(entry_id as usize).copied()
    }

    /// Get total number of entries
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// Get approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.offsets.len() * std::mem::size_of::<u64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_table_add_and_get() {
        let mut table = OffsetTable::new();

        table.add_offset(0, 0);
        table.add_offset(1, 150);
        table.add_offset(2, 320);

        assert_eq!(table.get_offset(0), Some(0));
        assert_eq!(table.get_offset(1), Some(150));
        assert_eq!(table.get_offset(2), Some(320));
        assert_eq!(table.get_offset(3), None);
    }

    #[test]
    fn test_offset_table_sparse_insert() {
        let mut table = OffsetTable::new();

        // Add offset for entry 10 (sparse)
        table.add_offset(10, 1000);

        assert_eq!(table.len(), 11); // 0-10
        assert_eq!(table.get_offset(10), Some(1000));
        assert_eq!(table.get_offset(5), Some(0)); // Default value
    }

    /// Story 6.2 (AC2): rkyv round-trip test with 100K+ entries for OffsetTable
    #[test]
    fn test_offset_table_rkyv_roundtrip_100k() {
        let mut table = OffsetTable::new();

        // Add 100,000 offsets simulating a large log file
        // Average line is ~200 bytes, so offsets grow by ~200 each
        let mut offset = 0u64;
        for i in 0..100_000u64 {
            table.add_offset(i, offset);
            offset += 150 + (i % 100); // Varying line lengths
        }

        // Serialize with rkyv
        let bytes = rkyv::to_bytes::<_, 256>(&table).expect("rkyv serialization failed");

        // 100K u64s = 800KB + overhead
        assert!(bytes.len() > 700_000, "Expected significant data for 100K entries");
        assert!(bytes.len() < 1_500_000, "Serialized data should be reasonably compact");

        // Validate archived data
        let archived = rkyv::check_archived_root::<OffsetTable>(&bytes)
            .expect("rkyv validation failed");

        // Verify archived vec has correct length
        assert_eq!(archived.offsets.len(), 100_000, "Archived should have 100K entries");

        // Deserialize back to regular struct
        let deserialized: OffsetTable = archived.deserialize(&mut rkyv::Infallible)
            .expect("rkyv deserialization failed");

        // Verify roundtrip preserves data
        assert_eq!(table.len(), deserialized.len(), "Length mismatch");

        // Spot check some offsets
        assert_eq!(table.get_offset(0), deserialized.get_offset(0), "First offset mismatch");
        assert_eq!(table.get_offset(50_000), deserialized.get_offset(50_000), "Middle offset mismatch");
        assert_eq!(table.get_offset(99_999), deserialized.get_offset(99_999), "Last offset mismatch");

        // Verify all offsets match
        for i in 0..100_000u64 {
            assert_eq!(table.get_offset(i), deserialized.get_offset(i),
                "Offset mismatch at entry {}", i);
        }
    }
}
