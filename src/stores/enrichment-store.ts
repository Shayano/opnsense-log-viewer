import { create } from 'zustand';

interface InterfaceMappingCache {
  mappings: Record<string, string>; // physical → logical
  lastUpdated: string; // ISO 8601 timestamp
  deviceId: string;
}

interface AliasMapping {
  aliasName: string;
  groupMembers: string[];
  description?: string;
  aliasType?: string;
}

// Story 3.5: Connection status types
type ConnectionStatus = 'connected' | 'disconnected' | 'degraded' | 'backup_enrichment';

interface ConnectionInfo {
  status: ConnectionStatus;
  lastError: string | null;
  lastChecked: string; // ISO 8601 timestamp
}

// Story 4.2: Backup enrichment metadata
interface BackupMetadata {
  importedAt: string;
  ageDays: number;
  sourceFile: string;
  exportTimestamp: string;
  deviceId: string;
}

interface EnrichmentStore {
  // Interface mappings
  interfaceMappings: Map<string, string>; // physical → logical
  lastUpdated: Date | null;
  deviceId: string | null;

  // Actions
  setInterfaceMappings: (cache: InterfaceMappingCache) => void;
  getLogicalName: (physicalName: string) => string | null;
  clearInterfaceMappings: () => void;

  // Rule labels (Story 3.3)
  ruleLabels: Map<string, string>; // hash → description
  ruleLabelsLastUpdated: Date | null;

  // Rule label actions
  setRuleLabels: (labels: Record<string, string>) => void;
  getRuleLabel: (hash: string) => string | null;
  addRuleLabel: (hash: string, description: string) => void;
  clearRuleLabels: () => void;

  // Aliases (Story 3.4)
  aliases: Map<string, AliasMapping[]>; // IP → aliases
  aliasesLastUpdated: Date | null;

  // Alias actions
  setAliases: (aliases: Record<string, AliasMapping[]>) => void;
  getAliasesForIP: (ip: string) => AliasMapping[] | null;
  addAlias: (ip: string, aliases: AliasMapping[]) => void;
  clearAliases: () => void;

  // Connection status (Story 3.5)
  connectionStatus: ConnectionStatus | null;
  lastError: string | null;
  lastChecked: Date | null;

  // Connection actions
  setConnectionStatus: (info: ConnectionInfo) => void;
  clearError: () => void;
  isConnected: () => boolean;

  // Backup enrichment (Story 4.2)
  backupEnrichmentActive: boolean;
  backupMetadata: BackupMetadata | null;

  // Backup enrichment actions
  setBackupEnrichment: (metadata: BackupMetadata) => void;
  clearBackupEnrichment: () => void;

  // Staleness indicator (Story 4.3)
  stalenessIndicatorDismissed: boolean;

  // Staleness indicator actions
  setStalenessIndicatorDismissed: (dismissed: boolean) => void;
  getStalenessIndicatorDismissed: () => boolean;
}

export const useEnrichmentStore = create<EnrichmentStore>((set, get) => ({
  // Initial state
  interfaceMappings: new Map(),
  lastUpdated: null,
  deviceId: null,

  // Set interface mappings from cache
  setInterfaceMappings: (cache) => {
    const mappingsMap = new Map(Object.entries(cache.mappings));
    set({
      interfaceMappings: mappingsMap,
      lastUpdated: new Date(cache.lastUpdated),
      deviceId: cache.deviceId,
    });
  },

  // Get logical name for a physical interface
  getLogicalName: (physicalName) => {
    const { interfaceMappings } = get();
    return interfaceMappings.get(physicalName) || null;
  },

  // Clear all mappings
  clearInterfaceMappings: () => {
    set({
      interfaceMappings: new Map(),
      lastUpdated: null,
      deviceId: null,
    });
  },

  // ============================================================================
  // Rule Labels (Story 3.3)
  // ============================================================================

  // Initial rule label state
  ruleLabels: new Map(),
  ruleLabelsLastUpdated: null,

  // Set rule labels from cache or API response
  setRuleLabels: (labels) => {
    const labelsMap = new Map(Object.entries(labels));
    set({
      ruleLabels: labelsMap,
      ruleLabelsLastUpdated: new Date(),
    });
  },

  // Get rule label for a specific hash
  getRuleLabel: (hash) => {
    const { ruleLabels } = get();
    return ruleLabels.get(hash) || null;
  },

  // Add single rule label
  addRuleLabel: (hash, description) => {
    const { ruleLabels } = get();
    const updatedLabels = new Map(ruleLabels);
    updatedLabels.set(hash, description);
    set({
      ruleLabels: updatedLabels,
      ruleLabelsLastUpdated: new Date(),
    });
  },

  // Clear all rule labels
  clearRuleLabels: () => {
    set({
      ruleLabels: new Map(),
      ruleLabelsLastUpdated: null,
    });
  },

  // ============================================================================
  // Aliases (Story 3.4)
  // ============================================================================

  // Initial alias state
  aliases: new Map(),
  aliasesLastUpdated: null,

  // Set aliases from cache or API response
  setAliases: (aliases) => {
    const aliasesMap = new Map(Object.entries(aliases));
    set({
      aliases: aliasesMap,
      aliasesLastUpdated: new Date(),
    });
  },

  // Get aliases for a specific IP
  getAliasesForIP: (ip) => {
    const { aliases } = get();
    return aliases.get(ip) || null;
  },

  // Add alias for single IP
  addAlias: (ip, aliasData) => {
    const { aliases } = get();
    const updatedAliases = new Map(aliases);
    updatedAliases.set(ip, aliasData);
    set({
      aliases: updatedAliases,
      aliasesLastUpdated: new Date(),
    });
  },

  // Clear all aliases
  clearAliases: () => {
    set({
      aliases: new Map(),
      aliasesLastUpdated: null,
    });
  },

  // ============================================================================
  // Connection Status (Story 3.5)
  // ============================================================================

  // Initial connection status
  connectionStatus: null,
  lastError: null,
  lastChecked: null,

  // Set connection status from backend
  setConnectionStatus: (info) => {
    set({
      connectionStatus: info.status,
      lastError: info.lastError,
      lastChecked: info.lastChecked ? new Date(info.lastChecked) : null,
    });
  },

  // Clear error message
  clearError: () => {
    set({ lastError: null });
  },

  // Check if currently connected
  isConnected: () => {
    const { connectionStatus } = get();
    return connectionStatus === 'connected';
  },

  // ============================================================================
  // Backup Enrichment (Story 4.2)
  // ============================================================================

  // Initial backup enrichment state
  backupEnrichmentActive: false,
  backupMetadata: null,

  // Set backup enrichment metadata
  setBackupEnrichment: (metadata) => {
    set({
      backupEnrichmentActive: true,
      backupMetadata: metadata,
      connectionStatus: 'backup_enrichment', // Update connection status to reflect backup mode
    });
  },

  // Clear backup enrichment
  clearBackupEnrichment: () => {
    set({
      backupEnrichmentActive: false,
      backupMetadata: null,
      stalenessIndicatorDismissed: false, // Reset dismissed state
    });
  },

  // ============================================================================
  // Staleness Indicator (Story 4.3)
  // ============================================================================

  // Initial staleness indicator state
  stalenessIndicatorDismissed: false,

  // Set staleness indicator dismissed state
  setStalenessIndicatorDismissed: (dismissed) => {
    set({ stalenessIndicatorDismissed: dismissed });
  },

  // Get staleness indicator dismissed state
  getStalenessIndicatorDismissed: () => {
    const { stalenessIndicatorDismissed } = get();
    return stalenessIndicatorDismissed;
  },
}));
