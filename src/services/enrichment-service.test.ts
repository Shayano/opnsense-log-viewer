import { describe, it, expect, beforeEach, vi } from 'vitest';
import { enrichAliases, loadCachedAliases } from './enrichment-service';
import { useEnrichmentStore } from '@/stores/enrichment-store';
import { invoke } from '@tauri-apps/api/core';
import { LogEntry } from '@/types/log-entry';
import toast from 'react-hot-toast';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    loading: vi.fn(() => 'toast-id'),
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Helper to create mock log entries
function createMockLogEntry(
  sourceIp: string,
  destinationIp: string,
  overrides: Partial<LogEntry> = {}
): LogEntry {
  return {
    timestamp: new Date().toISOString(),
    sourceIp,
    destinationIp,
    sourcePort: 12345,
    destinationPort: 80,
    protocol: 'TCP',
    action: 'pass',
    interface: 'lan',
    direction: 'out',
    ruleHash: 'abc123',
    ...overrides,
  };
}

describe('enrichment-service - IP Alias Resolution', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useEnrichmentStore.getState().clearAliases();
  });

  describe('enrichAliases', () => {
    it('should extract IPs and call invoke with correct parameters', async () => {
      const mockAliases = {
        '192.168.1.100': [
          {
            aliasName: 'Servers',
            groupMembers: ['192.168.1.100'],
            description: null,
            aliasType: 'host',
          },
        ],
      };

      vi.mocked(invoke).mockResolvedValue(mockAliases);

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
        createMockLogEntry('192.168.1.100', '10.0.0.2'), // Duplicate source IP
      ];

      await enrichAliases(entries);

      // Should extract 3 unique IPs: 192.168.1.100, 10.0.0.1, 10.0.0.2
      expect(invoke).toHaveBeenCalledWith('fetch_aliases', {
        ips: expect.arrayContaining(['192.168.1.100', '10.0.0.1', '10.0.0.2']),
      });
    });

    it('should update enrichment store with fetched aliases', async () => {
      const mockAliases = {
        '192.168.1.100': [
          {
            aliasName: 'Servers_Group',
            groupMembers: ['192.168.1.100', '192.168.1.101'],
            description: 'Server subnet',
            aliasType: 'network',
          },
        ],
        '10.0.0.1': [
          {
            aliasName: 'External_DNS',
            groupMembers: ['10.0.0.1'],
            description: null,
            aliasType: 'host',
          },
        ],
      };

      vi.mocked(invoke).mockResolvedValue(mockAliases);

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
      ];

      await enrichAliases(entries);

      // Check that aliases were stored in the store
      const storeAliases = useEnrichmentStore.getState().getAliasesForIP('192.168.1.100');
      expect(storeAliases).toHaveLength(1);
      expect(storeAliases![0].aliasName).toBe('Servers_Group');

      const externalDns = useEnrichmentStore.getState().getAliasesForIP('10.0.0.1');
      expect(externalDns).toHaveLength(1);
      expect(externalDns![0].aliasName).toBe('External_DNS');
    });

    it('should show success toast with correct counts', async () => {
      const mockAliases = {
        '192.168.1.100': [
          {
            aliasName: 'Servers',
            groupMembers: ['192.168.1.100'],
            description: null,
            aliasType: 'host',
          },
        ],
      };

      vi.mocked(invoke).mockResolvedValue(mockAliases);

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
        createMockLogEntry('192.168.1.101', '10.0.0.2'),
      ];

      await enrichAliases(entries);

      // 4 IPs total, 1 aliased
      expect(toast.loading).toHaveBeenCalledWith('Enriching aliases... 4 IPs');
      expect(toast.success).toHaveBeenCalledWith(
        'IP aliases enriched (1 of 4 aliased)',
        { id: 'toast-id' }
      );
    });

    it('should return early if no IPs found', async () => {
      const entries: LogEntry[] = [
        createMockLogEntry('', ''), // No IPs
      ];

      await enrichAliases(entries);

      expect(invoke).not.toHaveBeenCalled();
      expect(toast.loading).not.toHaveBeenCalled();
    });

    it('should handle API error with generic message', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Unknown error'));

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
      ];

      await enrichAliases(entries);

      expect(toast.error).toHaveBeenCalledWith(
        'Alias enrichment failed. Using cached data.'
      );
    });

    it('should handle API error with no credentials', async () => {
      vi.mocked(invoke).mockRejectedValue(
        new Error('No API credentials configured')
      );

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
      ];

      await enrichAliases(entries);

      expect(toast.error).toHaveBeenCalledWith(
        'Alias enrichment failed: No API credentials configured. Configure OPNsense connection in Settings.'
      );
    });

    it('should handle authentication error', async () => {
      vi.mocked(invoke).mockRejectedValue(
        new Error('Authentication failed (401)')
      );

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
      ];

      await enrichAliases(entries);

      expect(toast.error).toHaveBeenCalledWith(
        'Alias enrichment failed: Invalid API credentials. Check your API key and secret in Settings.'
      );
    });

    it('should handle network error', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Network timeout'));

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
      ];

      await enrichAliases(entries);

      expect(toast.error).toHaveBeenCalledWith(
        'Alias enrichment failed: Cannot reach OPNsense API. Check network connection.'
      );
    });

    it('should handle empty alias response (no aliases found)', async () => {
      vi.mocked(invoke).mockResolvedValue({});

      const entries: LogEntry[] = [
        createMockLogEntry('192.168.1.100', '10.0.0.1'),
      ];

      await enrichAliases(entries);

      expect(toast.success).toHaveBeenCalledWith(
        'IP aliases enriched (0 of 2 aliased)',
        { id: 'toast-id' }
      );
    });
  });

  describe('loadCachedAliases', () => {
    it('should load aliases from backend and store them', async () => {
      const mockAliases = {
        '192.168.1.100': [
          {
            aliasName: 'Servers',
            groupMembers: ['192.168.1.100'],
            description: null,
            aliasType: 'host',
          },
        ],
        '192.168.1.200': [
          {
            aliasName: 'Workstations',
            groupMembers: ['192.168.1.200'],
            description: null,
            aliasType: 'host',
          },
        ],
      };

      vi.mocked(invoke).mockResolvedValue(mockAliases);

      await loadCachedAliases();

      expect(invoke).toHaveBeenCalledWith('get_aliases');

      // Check that aliases were loaded into store
      const servers = useEnrichmentStore.getState().getAliasesForIP('192.168.1.100');
      expect(servers).toHaveLength(1);
      expect(servers![0].aliasName).toBe('Servers');

      const workstations = useEnrichmentStore.getState().getAliasesForIP('192.168.1.200');
      expect(workstations).toHaveLength(1);
      expect(workstations![0].aliasName).toBe('Workstations');
    });

    it('should not update store if no cached aliases', async () => {
      vi.mocked(invoke).mockResolvedValue({});

      await loadCachedAliases();

      expect(invoke).toHaveBeenCalledWith('get_aliases');

      // Store should remain empty
      const aliasesMap = useEnrichmentStore.getState().aliases;
      expect(aliasesMap.size).toBe(0);
    });

    it('should handle error gracefully (no toast, just console.error)', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      vi.mocked(invoke).mockRejectedValue(new Error('Failed to load'));

      await loadCachedAliases();

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Failed to load cached aliases:',
        expect.any(Error)
      );

      consoleErrorSpy.mockRestore();
    });

    it('should not show any toast notifications (silent load)', async () => {
      vi.mocked(invoke).mockResolvedValue({
        '192.168.1.100': [
          {
            aliasName: 'Test',
            groupMembers: ['192.168.1.100'],
            description: null,
            aliasType: null,
          },
        ],
      });

      await loadCachedAliases();

      expect(toast.loading).not.toHaveBeenCalled();
      expect(toast.success).not.toHaveBeenCalled();
      expect(toast.error).not.toHaveBeenCalled();
    });
  });
});
