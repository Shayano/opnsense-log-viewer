// Export types for Story 5.1: Filtered Result Export
// Story 5.2: Full Dataset Export with Streaming

export type ExportFormat = 'csv' | 'json';
export type ExportScope = 'filtered' | 'fullDataset';

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
  exportScope: ExportScope;
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

// Story 5.2: Export estimation types
export interface ExportEstimate {
  estimatedEntries: number;
  estimatedFileSizeMb: number;
  estimatedDurationSeconds: number;
}

export interface InsufficientDiskSpaceError {
  requiredMb: number;
  availableMb: number;
}

export interface ExportWarningModalProps {
  isOpen: boolean;
  estimate: ExportEstimate | null;
  onContinue: () => void;
  onCancel: () => void;
}
