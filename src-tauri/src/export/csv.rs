use crate::export::types::{ExportMetadata, ExportLogEntry, ExportResult, ExportScope};
use anyhow::{Context, Result};
use csv::Writer;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// CSV Exporter - Generates RFC 4180 compliant CSV files
pub struct CsvExporter {
    /// Metadata for export header
    metadata: ExportMetadata,
}

impl CsvExporter {
    /// Create new CSV exporter
    pub fn new(metadata: ExportMetadata) -> Self {
        Self { metadata }
    }

    /// Export log entries to CSV file
    pub fn export<P: AsRef<Path>>(
        &self,
        entries: Vec<ExportLogEntry>,
        save_path: P,
    ) -> Result<ExportResult> {
        let start = Instant::now();
        let save_path = save_path.as_ref();

        // Create CSV writer
        let file = File::create(save_path)
            .context("Failed to create CSV file")?;

        let mut writer = Writer::from_writer(file);

        // Write metadata header as comments
        self.write_metadata_header_impl(&mut writer)?;

        // Write column headers
        writer.write_record(&[
            "Timestamp",
            "Interface",
            "Source IP",
            "Source Port",
            "Destination IP",
            "Destination Port",
            "Protocol",
            "Action",
            "Rule Label",
        ])?;

        // Write data rows
        for entry in &entries {
            writer.write_record(&[
                &entry.timestamp,
                &entry.interface,
                &entry.source_ip,
                &entry.source_port.to_string(),
                &entry.destination_ip,
                &entry.destination_port.to_string(),
                &entry.protocol,
                &entry.action,
                &entry.rule_label,
            ])?;
        }

        // Flush writer
        writer.flush()?;

        // Calculate duration and file size
        let duration = start.elapsed();
        let file_size = std::fs::metadata(save_path)?.len();

        Ok(ExportResult {
            file_path: save_path.to_path_buf(),
            entries_written: entries.len(),
            duration_seconds: duration.as_secs_f64(),
            file_size_bytes: file_size,
        })
    }

    /// Export log entries with streaming (memory-efficient for large datasets)
    ///
    /// This uses BufWriter with 8KB buffer to prevent memory overflow.
    /// Writes data in chunks without loading entire dataset into memory.
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
            .context("Failed to create CSV file")?;
        let buf_writer = BufWriter::with_capacity(8192, file);
        let mut writer = Writer::from_writer(buf_writer);

        // Write metadata header as comments
        self.write_metadata_header_impl(&mut writer)?;

        // Write column headers
        writer.write_record(&[
            "Timestamp",
            "Interface",
            "Source IP",
            "Source Port",
            "Destination IP",
            "Destination Port",
            "Protocol",
            "Action",
            "Rule Label",
        ])?;

        // Stream data rows in chunks
        let mut entries_written = 0;
        const CHUNK_SIZE: usize = 5000;

        for (idx, entry) in entries.enumerate() {
            // Check cancellation every 1000 rows (lightweight check)
            if idx % 1000 == 0 && cancel_flag.load(Ordering::Relaxed) {
                // Clean up partial file
                drop(writer);
                std::fs::remove_file(save_path).ok();
                return Err(anyhow::anyhow!("Export cancelled by user"));
            }

            // Write entry row
            writer.write_record(&[
                &entry.timestamp,
                &entry.interface,
                &entry.source_ip,
                &entry.source_port.to_string(),
                &entry.destination_ip,
                &entry.destination_port.to_string(),
                &entry.protocol,
                &entry.action,
                &entry.rule_label,
            ])?;

            entries_written += 1;

            // Emit progress every CHUNK_SIZE rows
            if idx > 0 && idx % CHUNK_SIZE == 0 {
                writer.flush()?;  // Flush to disk
                progress_callback(entries_written, total_entries);
            }
        }

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

    /// Write metadata header as CSV comments (unified implementation)
    fn write_metadata_header_impl<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
    ) -> Result<()> {
        let meta = &self.metadata;

        // Exported by
        writer.write_record(&[format!(
            "# Exported by: {}",
            meta.exported_by
        )])?;

        // Export date
        writer.write_record(&[format!(
            "# Export Date: {}",
            meta.export_date.to_rfc3339()
        )])?;

        // Source file
        writer.write_record(&[format!(
            "# Source File: {} (SHA-256: {})",
            meta.source_file.path.display(),
            meta.source_file.sha256
        )])?;

        // Export scope (CRITICAL: Include for both streaming and non-streaming)
        let scope_str = match meta.export_scope {
            ExportScope::Filtered => "Filtered Results",
            ExportScope::FullDataset => "Full Dataset (unfiltered)",
        };
        writer.write_record(&[format!(
            "# Export Type: {}",
            scope_str
        )])?;

        // Filters applied
        if !meta.filters_applied.is_empty() {
            let filters_str = meta
                .filters_applied
                .iter()
                .map(|f| format!("{}={}", f.field, f.value))
                .collect::<Vec<_>>()
                .join(", ");

            writer.write_record(&[format!(
                "# Filters Applied: {}",
                filters_str
            )])?;
        }

        // Total entries
        writer.write_record(&[format!(
            "# Total Entries: {} of {}",
            meta.total_entries,
            meta.total_in_source
        )])?;

        // Enrichment status (if applicable)
        if let Some(enrichment) = &meta.enrichment_status {
            writer.write_record(&[format!(
                "# Enrichment: {}",
                enrichment.source
            )])?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use crate::export::types::SourceFileInfo;

    #[test]
    fn test_csv_export_basic() {
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
            ExportLogEntry {
                timestamp: "2026-01-15T14:31:00Z".to_string(),
                interface: "LAN".to_string(),
                source_ip: "192.168.1.50".to_string(),
                source_port: 12345,
                destination_ip: "8.8.8.8".to_string(),
                destination_port: 53,
                protocol: "UDP".to_string(),
                action: "pass".to_string(),
                rule_label: "Allow DNS".to_string(),
            },
        ];

        let temp_path = std::env::temp_dir().join("test_export.csv");
        let exporter = CsvExporter::new(metadata);
        let result = exporter.export(entries, &temp_path).unwrap();

        assert_eq!(result.entries_written, 2);
        assert!(result.file_size_bytes > 0);

        // Read file and verify contents
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("# Exported by: opnsense-log-viewer v1.0.0"));
        assert!(contents.contains("Timestamp,Interface,Source IP"));
        assert!(contents.contains("203.0.113.5"));
        assert!(contents.contains("Block RFC1918"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_csv_export_special_characters() {
        // Test that commas, quotes, newlines are properly escaped
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
                rule_label: "Block \"Malicious\" Traffic, from China".to_string(),
            },
        ];

        let temp_path = std::env::temp_dir().join("test_export_special.csv");
        let exporter = CsvExporter::new(metadata);
        let result = exporter.export(entries, &temp_path).unwrap();

        assert_eq!(result.entries_written, 1);

        // Read file and verify proper escaping
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        // CSV should quote the field and escape internal quotes
        assert!(contents.contains("\"Block \"\"Malicious\"\" Traffic, from China\""));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_csv_export_empty_entries() {
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

        let temp_path = std::env::temp_dir().join("test_export_empty.csv");
        let exporter = CsvExporter::new(metadata);
        let result = exporter.export(entries, &temp_path).unwrap();

        assert_eq!(result.entries_written, 0);

        // Read file and verify header exists
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("# Exported by:"));
        assert!(contents.contains("Timestamp,Interface,Source IP"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_csv_streaming_export_large_dataset() {
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

        let temp_path = std::env::temp_dir().join("test_streaming_export.csv");
        let exporter = CsvExporter::new(metadata);
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
        assert!(contents.contains("# Export Type: Full Dataset (unfiltered)"));
        assert!(contents.contains("# Total Entries: 10000"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_csv_streaming_cancellation() {
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

        let temp_path = std::env::temp_dir().join("test_cancel.csv");
        let exporter = CsvExporter::new(metadata);
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
    fn test_csv_streaming_filtered_scope() {
        // Test streaming export with Filtered scope
        let metadata = ExportMetadata {
            exported_by: "opnsense-log-viewer v1.0.0".to_string(),
            export_date: Utc::now(),
            source_file: SourceFileInfo {
                path: PathBuf::from("/path/to/filter.log"),
                sha256: "abc123".to_string(),
            },
            filters_applied: vec![],
            total_entries: 1000,
            total_in_source: 10000,
            export_scope: ExportScope::Filtered,
            enrichment_status: None,
        };

        let entries = (0..1000).map(|i| ExportLogEntry {
            timestamp: "2026-01-15T14:30:00Z".to_string(),
            interface: "WAN".to_string(),
            source_ip: "192.168.1.1".to_string(),
            source_port: 80,
            destination_ip: "10.0.0.1".to_string(),
            destination_port: 443,
            protocol: "TCP".to_string(),
            action: "block".to_string(),
            rule_label: format!("Rule {}", i),
        });

        let temp_path = std::env::temp_dir().join("test_streaming_filtered.csv");
        let exporter = CsvExporter::new(metadata);
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let result = exporter.export_streaming(
            entries,
            &temp_path,
            1000,
            |_, _| {},
            cancel_flag,
        ).unwrap();

        assert_eq!(result.entries_written, 1000);

        // Verify file contains correct scope
        let contents = std::fs::read_to_string(&temp_path).unwrap();
        assert!(contents.contains("# Export Type: Filtered Results"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }
}
