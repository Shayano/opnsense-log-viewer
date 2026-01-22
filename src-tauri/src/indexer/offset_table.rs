use serde::{Deserialize, Serialize};

/// Offset table for random access to raw log lines
/// Maps entry IDs to byte offsets in the source file
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Set offset directly (grows vector if needed)
    ///
    /// Story 6.3: Updated to resize vector when needed for progressive merge
    #[inline]
    pub fn set_offset(&mut self, entry_id: u64, offset: u64) {
        let idx = entry_id as usize;
        if idx >= self.offsets.len() {
            self.offsets.resize(idx + 1, 0);
        }
        self.offsets[idx] = offset;
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
}
