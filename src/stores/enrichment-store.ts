import { create } from 'zustand';

interface InterfaceMappingCache {
  mappings: Record<string, string>; // physical → logical
  lastUpdated: string; // ISO 8601 timestamp
  deviceId: string;
}

interface RuleLabelCache {
  mappings: Record<string, string>; // hash → description
  lastUpdated: string;
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
}));
