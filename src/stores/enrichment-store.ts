import { create } from 'zustand';

interface InterfaceMappingCache {
  mappings: Record<string, string>; // physical → logical
  lastUpdated: string; // ISO 8601 timestamp
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
}));
