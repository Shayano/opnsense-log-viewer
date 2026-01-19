use crate::export::types::{ExportMetadata, ExportLogEntry, ExportResult};
use anyhow::{Context, Result};
use csv::Writer;
use std::fs::File;
use std::path::Path;
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
        self.write_metadata_header(&mut writer)?;

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

    /// Write metadata header as CSV comments
    fn write_metadata_header<W: std::io::Write>(
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
}
