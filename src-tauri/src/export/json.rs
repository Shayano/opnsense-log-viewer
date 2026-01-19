use crate::export::types::{ExportMetadata, ExportLogEntry, ExportResult};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

    /// Export log entries with streaming (memory-efficient for large datasets)
    ///
    /// JSON streaming strategy:
    /// 1. Write metadata object upfront
    /// 2. Start entries array: write "["
    /// 3. Stream entries: serialize + write one at a time with commas
    /// 4. Close entries array: write "]"
    pub fn export_streaming<P: AsRef<Path>>(
        &self,
        entries: impl Iterator<Item = ExportLogEntry>,
        save_path: P,
        total_entries: usize,
        progress_callback: impl Fn(usize, usize),
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<ExportResult> {
        let start = Instant::now();
        let save_path = save_path.as_ref();

        // Create buffered writer (8KB buffer)
        let file = File::create(save_path)
            .context("Failed to create JSON file")?;
        let mut writer = BufWriter::with_capacity(8192, file);

        // Write opening brace and metadata
        writeln!(writer, "{{")?;
        write!(writer, "  \"metadata\": ")?;
        let metadata_json = serde_json::to_string_pretty(&self.metadata)?;
        // Indent metadata lines (except first line)
        for (i, line) in metadata_json.lines().enumerate() {
            if i > 0 {
                write!(writer, "  {}", line)?;
            } else {
                write!(writer, "{}", line)?;
            }
            if i < metadata_json.lines().count() - 1 {
                writeln!(writer)?;
            }
        }
        writeln!(writer, ",")?;
        writeln!(writer, "  \"entries\": [")?;

        // Stream entries one by one
        let mut entries_written = 0;
        const CHUNK_SIZE: usize = 5000;
        let mut first_entry = true;

        for (idx, entry) in entries.enumerate() {
            // Check cancellation every 1000 rows
            if idx % 1000 == 0 && cancel_flag.load(Ordering::Relaxed) {
                // Clean up partial file
                drop(writer);
                std::fs::remove_file(save_path).ok();
                return Err(anyhow::anyhow!("Export cancelled by user"));
            }

            // Write comma separator (except for first entry)
            if !first_entry {
                writeln!(writer, ",")?;
            }
            first_entry = false;

            // Serialize and write entry (4-space indent)
            let entry_json = serde_json::to_string(&entry)?;
            write!(writer, "    {}", entry_json)?;

            entries_written += 1;

            // Emit progress every CHUNK_SIZE rows
            if idx > 0 && idx % CHUNK_SIZE == 0 {
                writer.flush()?;
                progress_callback(entries_written, total_entries);
            }
        }

        // Close entries array and root object
        writeln!(writer)?;  // Newline after last entry
        writeln!(writer, "  ]")?;
        writeln!(writer, "}}")?;

        // Final flush
        writer.flush()?;

        // Calculate duration and file size
        let duration = start.elapsed();
        let file_size = std::fs::metadata(save_path)?.len();

        Ok(ExportResult {
            file_path: save_path.to_path_buf(),
            entries_written,
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
    use crate::export::types::{SourceFileInfo, ExportScope};

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
            export_scope: ExportScope::Filtered,
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
            export_scope: ExportScope::Filtered,
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
            export_scope: ExportScope::Filtered,
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
            export_scope: ExportScope::Filtered,
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

    #[test]
    fn test_json_streaming_export_large_dataset() {
        // Test streaming with 10K entries to verify memory efficiency
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 10000,
            total_in_source: 10000,
            export_scope: ExportScope::FullDataset,
            enrichment_status: None,
        };

        // Generate 10K test entries
        let entries = (0..10000).map(|i| ExportLogEntry {
            timestamp: format!("2026-01-15T14:{}:{:02}Z", i / 3600, (i % 3600) / 60),
            interface: format!("if{}", i % 5),
            source_ip: format!("192.168.{}.{}", i / 256, i % 256),
            source_port: (10000 + i as u16) % 65535,
            destination_ip: format!("10.0.{}.{}", i / 256, i % 256),
            destination_port: (443 + i as u16) % 65535,
            protocol: if i % 2 == 0 { "TCP".to_string() } else { "UDP".to_string() },
            action: if i % 3 == 0 { "block".to_string() } else { "pass".to_string() },
            rule_label: format!("Rule {}", i % 10),
        });

        let temp_path = std::env::temp_dir().join("test_streaming_export.json");
        let exporter = JsonExporter::new(metadata);
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let mut progress_calls = 0;
        let result = exporter.export_streaming(
            entries,
            &temp_path,
            10000,
            |current, total| {
                progress_calls += 1;
                assert!(current <= total);
            },
            cancel_flag,
        ).unwrap();

        assert_eq!(result.entries_written, 10000);
        assert!(result.file_size_bytes > 0);
        assert!(progress_calls > 0); // Progress callback should be called

        // Verify file contents
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("\"exportScope\": \"fullDataset\""));
        assert!(contents.contains("\"entries\":"));

        // Verify valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert!(parsed["metadata"].is_object());
        assert!(parsed["entries"].is_array());
        assert_eq!(parsed["entries"].as_array().unwrap().len(), 10000);

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_json_streaming_cancellation() {
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 10000,
            total_in_source: 10000,
            export_scope: ExportScope::FullDataset,
            enrichment_status: None,
        };

        let entries = (0..10000).map(|i| ExportLogEntry {
            timestamp: "2026-01-15T14:30:00Z".to_string(),
            interface: "WAN".to_string(),
            source_ip: "192.168.1.1".to_string(),
            source_port: 80,
            destination_ip: "10.0.0.1".to_string(),
            destination_port: 443,
            protocol: "TCP".to_string(),
            action: "pass".to_string(),
            rule_label: format!("Rule {}", i),
        });

        let temp_path = std::env::temp_dir().join("test_cancel.json");
        let exporter = JsonExporter::new(metadata);
        let cancel_flag = Arc::new(AtomicBool::new(false));

        // Set cancel flag immediately
        cancel_flag.store(true, Ordering::Relaxed);

        let result = exporter.export_streaming(
            entries,
            &temp_path,
            10000,
            |_, _| {},
            cancel_flag,
        );

        // Should error with cancellation message
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cancelled"));

        // Partial file should be cleaned up
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_json_streaming_empty_dataset() {
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 0,
            total_in_source: 10000,
            export_scope: ExportScope::Filtered,
            enrichment_status: None,
        };

        let entries = std::iter::empty();

        let temp_path = std::env::temp_dir().join("test_streaming_empty.json");
        let exporter = JsonExporter::new(metadata);
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let result = exporter.export_streaming(
            entries,
            &temp_path,
            0,
            |_, _| {},
            cancel_flag,
        ).unwrap();

        assert_eq!(result.entries_written, 0);

        // Verify valid JSON with empty array
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert!(parsed["metadata"].is_object());
        assert!(parsed["entries"].is_array());
        assert_eq!(parsed["entries"].as_array().unwrap().len(), 0);

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }
}
