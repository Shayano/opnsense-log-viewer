import { describe, it, expect, beforeEach, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';
import { exportEnrichmentData } from './enrichment-export';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
    dismiss: vi.fn(),
  },
}));

// Mock browser confirm
const mockConfirm = vi.fn();
global.confirm = mockConfirm;

describe('enrichment-export service', () => {
  const mockInvoke = invoke as any;
  const mockToastSuccess = vi.mocked(toast.success);
  const mockToastError = vi.mocked(toast.error);
  const mockToastDismiss = vi.mocked(toast.dismiss);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('exportEnrichmentData', () => {
    it('should export successfully with no warnings', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-2026-01-18.json',
        cacheStatus: {
          interfacesCount: 5,
          rulesCount: 120,
          aliasesCount: 15,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // export_enrichment_data
        .mockResolvedValueOnce('/path/to/saved/file.json'); // save_enrichment_export

      const result = await exportEnrichmentData();

      expect(result).toBe(true);
      expect(mockInvoke).toHaveBeenCalledWith('export_enrichment_data');
      expect(mockInvoke).toHaveBeenCalledWith('save_enrichment_export', {
        jsonData: mockExportResult.jsonData,
        filename: mockExportResult.filename,
      });
      expect(mockToastSuccess).toHaveBeenCalled();
      expect(mockConfirm).not.toHaveBeenCalled(); // No warning
    });

    it('should show warning and proceed when user confirms', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-2026-01-18.json',
        warning: 'Enrichment data is minimal',
        cacheStatus: {
          interfacesCount: 0,
          rulesCount: 2,
          aliasesCount: 0,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // export_enrichment_data
        .mockResolvedValueOnce('/path/to/saved/file.json'); // save_enrichment_export

      mockConfirm.mockReturnValueOnce(true); // User confirms

      const result = await exportEnrichmentData();

      expect(result).toBe(true);
      expect(mockConfirm).toHaveBeenCalledWith(
        expect.stringContaining('Enrichment data is minimal')
      );
      expect(mockConfirm).toHaveBeenCalledWith(expect.stringContaining('0 interfaces'));
      expect(mockConfirm).toHaveBeenCalledWith(expect.stringContaining('2 rule labels'));
      expect(mockConfirm).toHaveBeenCalledWith(expect.stringContaining('0 aliases'));
      expect(mockToastSuccess).toHaveBeenCalled();
    });

    it('should return false when user cancels warning', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-2026-01-18.json',
        warning: 'No enrichment data available',
        cacheStatus: {
          interfacesCount: 0,
          rulesCount: 0,
          aliasesCount: 0,
        },
      };

      mockInvoke.mockResolvedValueOnce(mockExportResult); // export_enrichment_data

      mockConfirm.mockReturnValueOnce(false); // User cancels

      const result = await exportEnrichmentData();

      expect(result).toBe(false);
      expect(mockInvoke).toHaveBeenCalledTimes(1); // Only export_enrichment_data
      expect(mockToastSuccess).not.toHaveBeenCalled();
      expect(mockToastError).not.toHaveBeenCalled();
    });

    it('should return false when user cancels save dialog', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-2026-01-18.json',
        cacheStatus: {
          interfacesCount: 3,
          rulesCount: 50,
          aliasesCount: 10,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // export_enrichment_data
        .mockRejectedValueOnce('Save dialog cancelled'); // save_enrichment_export

      const result = await exportEnrichmentData();

      expect(result).toBe(false);
      expect(mockToastError).not.toHaveBeenCalled(); // Not an error
    });

    it('should handle export preparation failure', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Failed to prepare export data'));

      const result = await exportEnrichmentData();

      expect(result).toBe(false);
      expect(mockToastError).toHaveBeenCalledWith(
        expect.stringContaining('Export failed')
      );
    });

    it('should handle save failure', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-2026-01-18.json',
        cacheStatus: {
          interfacesCount: 2,
          rulesCount: 30,
          aliasesCount: 5,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // export_enrichment_data
        .mockRejectedValueOnce(new Error('Failed to write file')); // save_enrichment_export

      const result = await exportEnrichmentData();

      expect(result).toBe(false);
      expect(mockToastError).toHaveBeenCalledWith(
        expect.stringContaining('Failed to write file')
      );
    });

    it('should include Open Folder button in success toast', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-2026-01-18.json',
        cacheStatus: {
          interfacesCount: 5,
          rulesCount: 100,
          aliasesCount: 20,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // export_enrichment_data
        .mockResolvedValueOnce('/path/to/exports/file.json'); // save_enrichment_export

      await exportEnrichmentData();

      // Verify toast.success was called with a function (for custom content)
      expect(mockToastSuccess).toHaveBeenCalled();
      const toastCall = mockToastSuccess.mock.calls[0];
      expect(typeof toastCall[0]).toBe('function'); // Custom toast content
      expect(toastCall[1]).toEqual({ duration: 5000 }); // Toast options
    });
  });

  describe('Performance Tests', () => {
    it('should handle large export data efficiently', async () => {
      // Simulate large export with 1000 interfaces, 5000 rules, 500 aliases
      const largeJsonData = JSON.stringify({
        interfaces: Object.fromEntries(
          Array.from({ length: 1000 }, (_, i) => [`vtnet${i}`, `Interface${i}`])
        ),
        rules: Object.fromEntries(
          Array.from({ length: 5000 }, (_, i) => [`hash${i}`, `Rule description ${i}`])
        ),
        aliases: Object.fromEntries(
          Array.from({ length: 500 }, (_, i) => [`10.0.${Math.floor(i / 255)}.${i % 255}`, [`alias${i}`]])
        ),
      });

      const mockExportResult = {
        jsonData: largeJsonData,
        filename: 'enrichment-export-large.json',
        cacheStatus: {
          interfacesCount: 1000,
          rulesCount: 5000,
          aliasesCount: 500,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult)
        .mockResolvedValueOnce('/path/to/large-export.json');

      const startTime = performance.now();
      const result = await exportEnrichmentData();
      const endTime = performance.now();

      expect(result).toBe(true);
      // Export should complete reasonably quickly (< 1 second for orchestration)
      expect(endTime - startTime).toBeLessThan(1000);
    });

    it('should handle empty export quickly', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export-empty.json',
        warning: 'No enrichment data available',
        cacheStatus: {
          interfacesCount: 0,
          rulesCount: 0,
          aliasesCount: 0,
        },
      };

      mockInvoke.mockResolvedValueOnce(mockExportResult);
      mockConfirm.mockReturnValueOnce(false); // User cancels

      const startTime = performance.now();
      await exportEnrichmentData();
      const endTime = performance.now();

      // Should be very fast when cancelled
      expect(endTime - startTime).toBeLessThan(100);
    });

    it('should handle multiple concurrent export attempts gracefully', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}, "aliases": {}}',
        filename: 'enrichment-export.json',
        cacheStatus: {
          interfacesCount: 10,
          rulesCount: 50,
          aliasesCount: 5,
        },
      };

      // Mock successful exports
      mockInvoke.mockImplementation(async (command: string) => {
        if (command === 'export_enrichment_data') {
          return mockExportResult;
        } else if (command === 'save_enrichment_export') {
          // Simulate save delay
          await new Promise((resolve) => setTimeout(resolve, 10));
          return '/path/to/export.json';
        }
      });

      // Start 3 concurrent exports
      const exports = [
        exportEnrichmentData(),
        exportEnrichmentData(),
        exportEnrichmentData(),
      ];

      const results = await Promise.all(exports);

      // All should succeed
      expect(results).toEqual([true, true, true]);
      // Each export should have called both commands
      expect(mockInvoke).toHaveBeenCalledTimes(6); // 3 * 2 commands
    });
  });

  describe('Integration Tests', () => {
    it('should complete full workflow: prepare → warn → confirm → save → toast', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {"vtnet0": "WAN"}, "rules": {"abc123": "Block RFC1918"}}',
        filename: 'enrichment-export-2026-01-18-142030.json',
        warning: 'Minimal enrichment data detected',
        cacheStatus: {
          interfacesCount: 1,
          rulesCount: 1,
          aliasesCount: 0,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // Step 1: Prepare
        .mockResolvedValueOnce('/home/user/exports/enrichment-export.json'); // Step 3: Save

      mockConfirm.mockReturnValueOnce(true); // Step 2: User confirms warning

      const result = await exportEnrichmentData();

      // Verify workflow
      expect(result).toBe(true);
      expect(mockInvoke).toHaveBeenNthCalledWith(1, 'export_enrichment_data');
      expect(mockConfirm).toHaveBeenCalledWith(
        expect.stringContaining('Minimal enrichment data detected')
      );
      expect(mockInvoke).toHaveBeenNthCalledWith(2, 'save_enrichment_export', {
        jsonData: mockExportResult.jsonData,
        filename: mockExportResult.filename,
      });
      expect(mockToastSuccess).toHaveBeenCalled();
    });

    it('should handle workflow interruption at warning step', async () => {
      const mockExportResult = {
        jsonData: '{}',
        filename: 'enrichment-export.json',
        warning: 'No data to export',
        cacheStatus: {
          interfacesCount: 0,
          rulesCount: 0,
          aliasesCount: 0,
        },
      };

      mockInvoke.mockResolvedValueOnce(mockExportResult); // Step 1: Prepare
      mockConfirm.mockReturnValueOnce(false); // Step 2: User cancels

      const result = await exportEnrichmentData();

      // Workflow should stop at warning
      expect(result).toBe(false);
      expect(mockInvoke).toHaveBeenCalledTimes(1); // Only prepare, no save
      expect(mockToastSuccess).not.toHaveBeenCalled();
      expect(mockToastError).not.toHaveBeenCalled();
    });

    it('should handle workflow interruption at save step', async () => {
      const mockExportResult = {
        jsonData: '{"interfaces": {}, "rules": {}}',
        filename: 'enrichment-export.json',
        cacheStatus: {
          interfacesCount: 2,
          rulesCount: 10,
          aliasesCount: 1,
        },
      };

      mockInvoke
        .mockResolvedValueOnce(mockExportResult) // Step 1: Prepare
        .mockRejectedValueOnce('Save dialog cancelled'); // Step 2: User cancels save

      const result = await exportEnrichmentData();

      // Workflow should stop at save dialog
      expect(result).toBe(false);
      expect(mockInvoke).toHaveBeenCalledTimes(2);
      expect(mockToastSuccess).not.toHaveBeenCalled();
      expect(mockToastError).not.toHaveBeenCalled(); // Cancellation is not an error
    });
  });
});
