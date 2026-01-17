use std::collections::HashMap;

/// Inverted index for high-cardinality fields (IPs, ports)
/// Maps field values to lists of entry IDs containing that value
pub struct InvertedIndex {
    source_ips: HashMap<String, Vec<u64>>,
    dest_ips: HashMap<String, Vec<u64>>,
    source_ports: HashMap<u16, Vec<u64>>,
    dest_ports: HashMap<u16, Vec<u64>>,
}

impl Default for InvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            source_ips: HashMap::new(),
            dest_ips: HashMap::new(),
            source_ports: HashMap::new(),
            dest_ports: HashMap::new(),
        }
    }

    /// Add an entry to the inverted index
    pub fn add_entry(
        &mut self,
        entry_id: u64,
        source_ip: Option<&str>,
        dest_ip: Option<&str>,
        source_port: Option<u16>,
        dest_port: Option<u16>,
    ) {
        if let Some(ip) = source_ip {
            self.source_ips
                .entry(ip.to_string())
                .or_default()
                .push(entry_id);
        }

        if let Some(ip) = dest_ip {
            self.dest_ips
                .entry(ip.to_string())
                .or_default()
                .push(entry_id);
        }

        if let Some(port) = source_port {
            self.source_ports
                .entry(port)
                .or_default()
                .push(entry_id);
        }

        if let Some(port) = dest_port {
            self.dest_ports
                .entry(port)
                .or_default()
                .push(entry_id);
        }
    }

    /// Query source IPs - returns entry IDs matching the given IP
    pub fn query_source_ip(&self, ip: &str) -> Option<&Vec<u64>> {
        self.source_ips.get(ip)
    }

    /// Query destination IPs - returns entry IDs matching the given IP
    pub fn query_dest_ip(&self, ip: &str) -> Option<&Vec<u64>> {
        self.dest_ips.get(ip)
    }

    /// Query source ports - returns entry IDs matching the given port
    pub fn query_source_port(&self, port: u16) -> Option<&Vec<u64>> {
        self.source_ports.get(&port)
    }

    /// Query destination ports - returns entry IDs matching the given port
    pub fn query_dest_port(&self, port: u16) -> Option<&Vec<u64>> {
        self.dest_ports.get(&port)
    }

    /// Get approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let mut size = 0;

        // HashMap overhead + entries
        size += self.source_ips.capacity() * (std::mem::size_of::<String>() + std::mem::size_of::<Vec<u64>>());
        size += self.dest_ips.capacity() * (std::mem::size_of::<String>() + std::mem::size_of::<Vec<u64>>());
        size += self.source_ports.capacity() * (std::mem::size_of::<u16>() + std::mem::size_of::<Vec<u64>>());
        size += self.dest_ports.capacity() * (std::mem::size_of::<u16>() + std::mem::size_of::<Vec<u64>>());

        // Actual data in vectors
        for (key, ids) in &self.source_ips {
            size += key.len() + ids.len() * std::mem::size_of::<u64>();
        }
        for (key, ids) in &self.dest_ips {
            size += key.len() + ids.len() * std::mem::size_of::<u64>();
        }
        for ids in self.source_ports.values() {
            size += ids.len() * std::mem::size_of::<u64>();
        }
        for ids in self.dest_ports.values() {
            size += ids.len() * std::mem::size_of::<u64>();
        }

        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverted_index_insert_and_query() {
        let mut index = InvertedIndex::new();

        // Add entry 1
        index.add_entry(1, Some("192.168.1.100"), Some("10.0.0.5"), Some(443), Some(80));

        // Add entry 2 with same source IP
        index.add_entry(2, Some("192.168.1.100"), Some("10.0.0.6"), Some(22), Some(80));

        // Query source IP
        let results = index.query_source_ip("192.168.1.100");
        assert_eq!(results, Some(&vec![1, 2]));

        // Query dest IP
        let results = index.query_dest_ip("10.0.0.5");
        assert_eq!(results, Some(&vec![1]));

        // Query source port
        let results = index.query_source_port(443);
        assert_eq!(results, Some(&vec![1]));

        // Query dest port (shared)
        let results = index.query_dest_port(80);
        assert_eq!(results, Some(&vec![1, 2]));

        // Query non-existent
        let results = index.query_source_ip("192.168.1.1");
        assert_eq!(results, None);
    }

    #[test]
    fn test_memory_usage_tracking() {
        let mut index = InvertedIndex::new();
        let initial_usage = index.memory_usage();

        // Add 1000 entries
        for i in 0..1000 {
            index.add_entry(
                i,
                Some(&format!("192.168.1.{}", i % 255)),
                Some(&format!("10.0.0.{}", i % 255)),
                Some((i % 65535) as u16),
                Some((i % 65535) as u16),
            );
        }

        let final_usage = index.memory_usage();
        assert!(final_usage > initial_usage);

        // Ensure memory usage is reasonable (<250 MB for inverted index portion)
        assert!(final_usage < 250 * 1024 * 1024);
    }
}
