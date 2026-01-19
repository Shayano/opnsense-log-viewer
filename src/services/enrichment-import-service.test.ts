import { describe, it, expect, beforeEach, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';
import { importEnrichmentData } from './enrichment-import-service';
import { useEnrichmentStore } from '@/stores/enrichment-store';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Mock console methods to prevent noise and unhandled errors
const mockConsoleError = vi.fn();
const mockConsoleWarn = vi.fn();
const originalConsoleError = console.error;
const originalConsoleWarn = console.warn;

// Mock browser alert and confirm
const mockAlert = vi.fn();
const mockConfirm = vi.fn();
const originalAlert = global.alert;
const originalConfirm = global.confirm;

describe('enrichment-import-service', () => {
  const mockInvoke = invoke as any;

  beforeEach(() => {
    // Setup mocks
    global.alert = mockAlert;
    global.confirm = mockConfirm;
    console.error = mockConsoleError;
    console.warn = mockConsoleWarn;
    vi.clearAllMocks();
    useEnrichmentStore.setState({
      interfaceMappings: new Map(),
      ruleLabels: new Map(),
      aliases: new Map(),
      backupEnrichmentActive: false,
      backupMetadata: null,
    });
  });

  afterEach(() => {
    // Restore original globals to prevent test pollution
    global.alert = originalAlert;
    global.confirm = originalConfirm;
    console.error = originalConsoleError;
    console.warn = originalConsoleWarn;
  });

  describe('importEnrichmentData', () => {
    it('should return false when user cancels file picker', async () => {
      mockInvoke.mockResolvedValueOnce(null); // File picker returns null

      const result = await importEnrichmentData();

      expect(result).toBe(false);
      expect(mockInvoke).toHaveBeenCalledWith('open_enrichment_file_picker');
      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(toast.success).not.toHaveBeenCalled();
    });

    it('should show validation error and return false when validation fails', async () => {
      mockInvoke
        .mockResolvedValueOnce('/path/to/invalid.json') // File picker
        .mockResolvedValueOnce({
          // Validation
          isValid: false,
          errorMessage: 'Missing required fields',
          missingFields: ['interfaceMappings', 'ruleLabels'],
          isStale: false,
        });

      const result = await importEnrichmentData();

      expect(result).toBe(false);
      expect(mockAlert).toHaveBeenCalledWith(
        expect.stringContaining('Missing required fields')
      );
      expect(mockAlert).toHaveBeenCalledWith(
        expect.stringContaining('Missing fields: interfaceMappings, ruleLabels')
      );
      expect(toast.success).not.toHaveBeenCalled();
    });

    it('should import successfully when validation passes and data is fresh', async () => {
      const mockFilePath = '/path/to/enrichment.json';
      const mockImportResult = {
        interfacesImported: 5,
        rulesImported: 120,
        aliasesImported: 15,
        exportTimestamp: '2026-01-18T12:00:00Z',
        deviceId: 'test-device-123',
        ageDays: 1,
      };

      mockInvoke
        .mockResolvedValueOnce(mockFilePath) // File picker
        .mockResolvedValueOnce({
          // Validation
          isValid: true,
          isStale: false,
          ageDays: 1,
          metadata: {
            exportTimestamp: '2026-01-18T12:00:00Z',
            deviceId: 'test-device-123',
            opnsenseVersion: '24.1',
            configHash: 'abc123',
            appVersion: '0.1.0',
            dataSource: 'api',
            cacheStatus: {
              interfacesCount: 5,
              rulesCount: 120,
              aliasesCount: 15,
            },
          },
        })
        .mockResolvedValueOnce(mockImportResult) // Import
        .mockResolvedValueOnce({ mappings: { vtnet0: 'WAN', vtnet1: 'LAN' } }) // Interface mappings
        .mockResolvedValueOnce({ rule1: 'Allow HTTPS', rule2: 'Block RFC1918' }) // Rule labels
        .mockResolvedValueOnce({ alias1: { addresses: ['10.0.0.0/8'] } }); // Aliases

      const result = await importEnrichmentData();

      expect(result).toBe(true);
      expect(mockConfirm).not.toHaveBeenCalled(); // No staleness warning
      expect(toast.success).toHaveBeenCalledWith(
        expect.stringContaining('5 interfaces, 120 rules, 15 aliases'),
        { duration: 4000 }
      );

      // Verify store was updated
      const state = useEnrichmentStore.getState();
      expect(state.backupMetadata).toBeDefined();
      expect(state.backupMetadata?.ageDays).toBe(1);
      expect(state.backupMetadata?.deviceId).toBe('test-device-123');
      expect(state.ruleLabels.size).toBe(2);
      expect(state.aliases.size).toBe(1);
    });

    it('should show staleness warning and continue when user confirms', async () => {
      const mockFilePath = '/path/to/old-enrichment.json';
      const mockImportResult = {
        interfacesImported: 3,
        rulesImported: 80,
        aliasesImported: 10,
        exportTimestamp: '2026-01-05T12:00:00Z',
        deviceId: 'test-device-456',
        ageDays: 14,
      };

      mockInvoke
        .mockResolvedValueOnce(mockFilePath) // File picker
        .mockResolvedValueOnce({
          // Validation
          isValid: true,
          isStale: true,
          ageDays: 14,
          metadata: {
            exportTimestamp: '2026-01-05T12:00:00Z',
            deviceId: 'test-device-456',
            opnsenseVersion: '23.7',
            configHash: 'xyz789',
            appVersion: '0.1.0',
            dataSource: 'backup',
            cacheStatus: {
              interfacesCount: 3,
              rulesCount: 80,
              aliasesCount: 10,
            },
          },
        });

      mockConfirm.mockReturnValueOnce(true); // User confirms staleness warning

      mockInvoke
        .mockResolvedValueOnce(mockImportResult) // Import
        .mockResolvedValueOnce({ mappings: { vtnet0: 'WAN' } }) // Interface mappings
        .mockResolvedValueOnce({ rule1: 'Allow SSH' }) // Rule labels
        .mockResolvedValueOnce({ alias1: { addresses: ['192.168.0.0/16'] } }); // Aliases

      const result = await importEnrichmentData();

      expect(result).toBe(true);
      expect(mockConfirm).toHaveBeenCalledWith(expect.stringContaining('14 days old'));
      expect(toast.success).toHaveBeenCalledWith(
        expect.stringContaining('3 interfaces, 80 rules, 10 aliases'),
        { duration: 4000 }
      );

      // Verify backup metadata includes staleness
      const state = useEnrichmentStore.getState();
      expect(state.backupMetadata?.ageDays).toBe(14);
    });

    it('should return false when user cancels staleness warning', async () => {
      mockInvoke
        .mockResolvedValueOnce('/path/to/old-enrichment.json') // File picker
        .mockResolvedValueOnce({
          // Validation
          isValid: true,
          isStale: true,
          ageDays: 30,
          metadata: {
            exportTimestamp: '2025-12-20T12:00:00Z',
            deviceId: 'old-device',
            opnsenseVersion: '23.1',
            configHash: 'old123',
            appVersion: '0.1.0',
            dataSource: 'backup',
            cacheStatus: {
              interfacesCount: 2,
              rulesCount: 50,
              aliasesCount: 5,
            },
          },
        });

      mockConfirm.mockReturnValueOnce(false); // User cancels

      const result = await importEnrichmentData();

      expect(result).toBe(false);
      expect(mockConfirm).toHaveBeenCalledWith(expect.stringContaining('30 days old'));
      expect(mockInvoke).toHaveBeenCalledTimes(2); // Only file picker + validation
      expect(toast.success).not.toHaveBeenCalled();
    });

    it('should handle import errors gracefully', async () => {
      mockInvoke
        .mockResolvedValueOnce('/path/to/enrichment.json') // File picker
        .mockResolvedValueOnce({ isValid: true, isStale: false }) // Validation
        .mockRejectedValueOnce(new Error('Failed to read file')); // Import fails

      const result = await importEnrichmentData();

      expect(result).toBe(false);
      expect(toast.error).toHaveBeenCalledWith(
        expect.stringContaining('Could not read file')
      );
    });

    it('should handle validation error in import gracefully', async () => {
      mockInvoke
        .mockResolvedValueOnce('/path/to/corrupted.json') // File picker
        .mockResolvedValueOnce({ isValid: true, isStale: false }) // Validation passes
        .mockRejectedValueOnce(new Error('Invalid JSON structure')); // Import fails with validation error

      const result = await importEnrichmentData();

      expect(result).toBe(false);
      expect(toast.error).toHaveBeenCalledWith(
        expect.stringContaining('Import validation failed')
      );
    });

    it('should continue import even if reload fails', async () => {
      const mockFilePath = '/path/to/enrichment.json';
      const mockImportResult = {
        interfacesImported: 2,
        rulesImported: 40,
        aliasesImported: 5,
        exportTimestamp: '2026-01-18T12:00:00Z',
        deviceId: 'device-123',
        ageDays: 1,
      };

      mockInvoke
        .mockResolvedValueOnce(mockFilePath) // File picker
        .mockResolvedValueOnce({ isValid: true, isStale: false }) // Validation
        .mockResolvedValueOnce(mockImportResult) // Import succeeds
        .mockRejectedValueOnce(new Error('Failed to reload interface mappings')) // Reload fails
        .mockRejectedValueOnce(new Error('Failed to reload rule labels'))
        .mockRejectedValueOnce(new Error('Failed to reload aliases'));

      const result = await importEnrichmentData();

      // Should still succeed even though reload failed
      expect(result).toBe(true);
      expect(toast.success).toHaveBeenCalledWith(
        expect.stringContaining('2 interfaces, 40 rules, 5 aliases'),
        { duration: 4000 }
      );

      // Verify backup metadata was still set
      const state = useEnrichmentStore.getState();
      expect(state.backupMetadata).toBeDefined();
      expect(state.backupMetadata?.ageDays).toBe(1);
    });

    it('should handle generic errors', async () => {
      mockInvoke
        .mockResolvedValueOnce('/path/to/enrichment.json') // File picker
        .mockRejectedValueOnce('Unknown error'); // Unexpected error type

      const result = await importEnrichmentData();

      expect(result).toBe(false);
      expect(toast.error).toHaveBeenCalledWith(expect.stringContaining('Import failed'));
    });
  });
});
