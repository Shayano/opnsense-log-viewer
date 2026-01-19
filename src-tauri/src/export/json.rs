use crate::export::types::{ExportMetadata, ExportLogEntry, ExportResult};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

/// JSON Exporter - Generates pretty-printed JSON files
pub struct JsonExporter {
    /// Metadata for export
    metadata: ExportMetadata,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExportDocument {
    metadata: ExportMetadata,
    entries: Vec<ExportLogEntry>,
}

impl JsonExporter {
    /// Create new JSON exporter
    pub fn new(metadata: ExportMetadata) -> Self {
        Self { metadata }
    }

    /// Export log entries to JSON file
    pub fn export<P: AsRef<Path>>(
        &self,
        entries: Vec<ExportLogEntry>,
        save_path: P,
    ) -> Result<ExportResult> {
        let start = Instant::now();
        let save_path = save_path.as_ref();

        // Build JSON document
        let entries_count = entries.len();
        let document = JsonExportDocument {
            metadata: self.metadata.clone(),
            entries,
        };

        // Write to file with pretty-print (2-space indent)
        let file = File::create(save_path)
            .context("Failed to create JSON file")?;

        serde_json::to_writer_pretty(file, &document)
            .context("Failed to write JSON")?;

        // Calculate duration and file size
        let duration = start.elapsed();
        let file_size = std::fs::metadata(save_path)?.len();

        Ok(ExportResult {
            file_path: save_path.to_path_buf(),
            entries_written: entries_count,
            duration_seconds: duration.as_secs_f64(),
            file_size_bytes: file_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use crate::export::types::SourceFileInfo;

    #[test]
    fn test_json_export_basic() {
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 2,
            total_in_source: 1000,
            enrichment_status: None,
        };

        let entries = vec![
            ExportLogEntry {
                timestamp: "2026-01-15T14:30:00Z".to_string(),
                interface: "WAN".to_string(),
                source_ip: "203.0.113.5".to_string(),
                source_port: 54321,
                destination_ip: "192.168.1.100".to_string(),
                destination_port: 443,
                protocol: "TCP".to_string(),
                action: "block".to_string(),
                rule_label: "Block RFC1918".to_string(),
            },
        ];

        let temp_path = std::env::temp_dir().join("test_export.json");
        let exporter = JsonExporter::new(metadata);
        let result = exporter.export(entries, &temp_path).unwrap();

        assert_eq!(result.entries_written, 1);
        assert!(result.file_size_bytes > 0);

        // Read file and verify contents
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("\"exportedBy\": \"opnsense-log-viewer v1.0.0\""));
        assert!(contents.contains("\"entries\":"));
        assert!(contents.contains("\"sourceIp\": \"203.0.113.5\""));
        assert!(contents.contains("\"ruleLabel\": \"Block RFC1918\""));

        // Verify pretty-print (2-space indent)
        assert!(contents.contains("  \"metadata\":"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_json_export_camel_case() {
        // Verify all fields use camelCase
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 1,
            total_in_source: 1000,
            enrichment_status: None,
        };

        let entries = vec![
            ExportLogEntry {
                timestamp: "2026-01-15T14:30:00Z".to_string(),
                interface: "WAN".to_string(),
                source_ip: "203.0.113.5".to_string(),
                source_port: 54321,
                destination_ip: "192.168.1.100".to_string(),
                destination_port: 443,
                protocol: "TCP".to_string(),
                action: "block".to_string(),
                rule_label: "Block RFC1918".to_string(),
            },
        ];

        let temp_path = std::env::temp_dir().join("test_export_camelcase.json");
        let exporter = JsonExporter::new(metadata);
        exporter.export(entries, &temp_path).unwrap();

        // Read and verify camelCase
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("\"exportedBy\":"));
        assert!(contents.contains("\"exportDate\":"));
        assert!(contents.contains("\"sourceFile\":"));
        assert!(contents.contains("\"filtersApplied\":"));
        assert!(contents.contains("\"totalEntries\":"));
        assert!(contents.contains("\"totalInSource\":"));
        assert!(contents.contains("\"sourceIp\":"));
        assert!(contents.contains("\"sourcePort\":"));
        assert!(contents.contains("\"destinationIp\":"));
        assert!(contents.contains("\"destinationPort\":"));
        assert!(contents.contains("\"ruleLabel\":"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_json_export_empty_entries() {
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 0,
            total_in_source: 1000,
            enrichment_status: None,
        };

        let entries = vec![];

        let temp_path = std::env::temp_dir().join("test_export_empty.json");
        let exporter = JsonExporter::new(metadata);
        let result = exporter.export(entries, &temp_path).unwrap();

        assert_eq!(result.entries_written, 0);

        // Read file and verify valid JSON with empty array
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("\"metadata\":"));
        assert!(contents.contains("\"entries\": []"));

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert!(parsed["entries"].is_array());
        assert_eq!(parsed["entries"].as_array().unwrap().len(), 0);

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_json_export_special_characters() {
        // Test that special characters are properly escaped in JSON
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 1,
            total_in_source: 1000,
            enrichment_status: None,
        };

        let entries = vec![
            ExportLogEntry {
                timestamp: "2026-01-15T14:30:00Z".to_string(),
                interface: "WAN".to_string(),
                source_ip: "203.0.113.5".to_string(),
                source_port: 80,
                destination_ip: "192.168.1.100".to_string(),
                destination_port: 443,
                protocol: "TCP".to_string(),
                action: "block".to_string(),
                rule_label: "Block \"Malicious\" Traffic\nwith newlines".to_string(),
            },
        ];

        let temp_path = std::env::temp_dir().join("test_export_special_json.json");
        let exporter = JsonExporter::new(metadata);
        exporter.export(entries, &temp_path).unwrap();

        // Read file and verify proper JSON escaping
        let contents = std::fs::read_to_string(&temp_path).unwrap();

        // Verify it's valid JSON (will fail if not properly escaped)
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(
            parsed["entries"][0]["ruleLabel"].as_str().unwrap(),
            "Block \"Malicious\" Traffic\nwith newlines"
        );

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }
}
