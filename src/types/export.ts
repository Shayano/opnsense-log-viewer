// Export types for Story 5.1: Filtered Result Export

export type ExportFormat = 'csv' | 'json';

export interface SourceFileInfo {
  path: string;
  sha256: string;
}

export interface FilterInfo {
  field: string;
  operator: string;
  value: string;
}

export interface EnrichmentInfo {
  active: boolean;
  source: string;
  exportedAt?: string;
}

export interface ExportMetadata {
  exportedBy: string;
  exportDate: string;
  sourceFile: SourceFileInfo;
  filtersApplied: FilterInfo[];
  totalEntries: number;
  totalInSource: number;
  enrichmentStatus?: EnrichmentInfo;
}

export interface ExportProgress {
  current: number;
  total: number;
  status: string;
  rowsPerSecond: number;
  elapsedSeconds: number;
}

export interface ExportResult {
  filePath: string;
  entriesWritten: number;
  durationSeconds: number;
  fileSizeBytes: number;
}

export interface ExportLogEntry {
  timestamp: string;
  interface: string;
  sourceIp: string;
  sourcePort: number;
  destinationIp: string;
  destinationPort: number;
  protocol: string;
  action: string;
  ruleLabel: string;
}

export interface ExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  totalEntries: number;
  onExport: (format: ExportFormat) => void;
}

export interface ExportProgressModalProps {
  isOpen: boolean;
  progress: ExportProgress | null;
  onCancel: () => void;
}
