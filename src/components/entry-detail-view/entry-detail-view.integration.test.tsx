import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { EntryDetailView } from './entry-detail-view';
import { Action, Protocol, type LogEntry } from '@/types/log-entry';
import * as clipboard from '@/utils/clipboard';
import toast from 'react-hot-toast';

// Mock clipboard
vi.mock('@/utils/clipboard');

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('Entry Detail View Integration Tests', () => {
  const mockCopyToClipboard = vi.mocked(clipboard.copyToClipboard);
  const mockToastSuccess = vi.mocked(toast.success);
  const mockToastError = vi.mocked(toast.error);
  const mockOnClose = vi.fn();

  const mockEntry: LogEntry = {
    id: '1',
    timestamp: '2026-01-18T12:00:00Z',
    interface: 'vtnet0',
    sourceIp: '192.168.1.100',
    sourcePort: 443,
    destinationIp: '10.0.0.1',
    destinationPort: 80,
    protocol: Protocol.TCP,
    action: Action.BLOCK,
    ruleLabel: 'abc123',
    rawLine:
      'Jan 18 12:00:00 firewall filterlog: 5,,,1000000103,vtnet0,match,block,in,4,0x0,,64,0,0,DF,6,tcp,52,192.168.1.100,10.0.0.1,443,80,0,S,123456789,,1024,,mss',
  };

  const mockEntry2: LogEntry = {
    id: '2',
    timestamp: '2026-01-18T12:01:00Z',
    interface: 'vtnet1',
    sourceIp: '10.0.0.50',
    sourcePort: 22,
    destinationIp: '192.168.1.200',
    destinationPort: 443,
    protocol: Protocol.UDP,
    action: Action.PASS,
    ruleLabel: 'def456',
    rawLine:
      'Jan 18 12:01:00 firewall filterlog: 5,,,1000000104,vtnet1,match,pass,in,4,0x0,,64,0,0,DF,17,udp,52,10.0.0.50,192.168.1.200,22,443',
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Copy-to-Clipboard Integration', () => {
    it('should handle multiple copy operations in sequence', async () => {
      mockCopyToClipboard.mockResolvedValue(true);

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const copyButton = screen.getByRole('button', { name: /copy full entry/i });

      // First copy
      fireEvent.click(copyButton);
      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(mockEntry.rawLine);
        expect(mockToastSuccess).toHaveBeenCalledWith('Copied to clipboard');
      });

      vi.clearAllMocks();

      // Second copy
      fireEvent.click(copyButton);
      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(mockEntry.rawLine);
        expect(mockToastSuccess).toHaveBeenCalledWith('Copied to clipboard');
      });
    });

    it('should handle alternating success and failure', async () => {
      mockCopyToClipboard
        .mockResolvedValueOnce(true)
        .mockResolvedValueOnce(false)
        .mockResolvedValueOnce(true);

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const copyButton = screen.getByRole('button', { name: /copy full entry/i });

      // First: success
      fireEvent.click(copyButton);
      await waitFor(() => {
        expect(mockToastSuccess).toHaveBeenCalledWith('Copied to clipboard');
      });

      vi.clearAllMocks();

      // Second: failure
      fireEvent.click(copyButton);
      await waitFor(() => {
        expect(mockToastError).toHaveBeenCalledWith('Failed to copy to clipboard');
      });

      vi.clearAllMocks();

      // Third: success again
      fireEvent.click(copyButton);
      await waitFor(() => {
        expect(mockToastSuccess).toHaveBeenCalledWith('Copied to clipboard');
      });
    });
  });

  describe('Open/Close/Reopen Workflow', () => {
    it('should handle open → close → reopen workflow', () => {
      const { rerender } = render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      // Verify open
      expect(screen.getByRole('complementary')).toBeInTheDocument();

      // Close
      const closeButton = screen.getByRole('button', { name: /close detail view/i });
      fireEvent.click(closeButton);
      expect(mockOnClose).toHaveBeenCalled();

      // Rerender as closed
      rerender(
        <EntryDetailView
          entry={mockEntry}
          isOpen={false}
          onClose={mockOnClose}
        />
      );

      expect(screen.queryByRole('complementary')).not.toBeInTheDocument();

      // Reopen with different entry
      rerender(
        <EntryDetailView
          entry={mockEntry2}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(screen.getByRole('complementary')).toBeInTheDocument();
      expect(screen.getByText('192.168.1.200')).toBeInTheDocument();
    });

    it('should handle Escape key to close', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      fireEvent.keyDown(document, { key: 'Escape' });
      expect(mockOnClose).toHaveBeenCalled();
    });

    it('should not close when clicking inside detail pane', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const detailView = screen.getByRole('complementary');
      fireEvent.mouseDown(detailView);

      expect(mockOnClose).not.toHaveBeenCalled();
    });
  });

  describe('Complete Integration Workflow', () => {
    it('should handle full workflow: open → copy → close → reopen → copy', async () => {
      mockCopyToClipboard.mockResolvedValue(true);

      // Step 1: Open with first entry
      const { rerender } = render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      // Step 2: Verify entry data
      expect(screen.getByText('10.0.0.1')).toBeInTheDocument();
      expect(screen.getByText('abc123')).toBeInTheDocument();

      // Step 3: Copy to clipboard
      const copyButton = screen.getByRole('button', { name: /copy full entry/i });
      fireEvent.click(copyButton);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(mockEntry.rawLine);
        expect(mockToastSuccess).toHaveBeenCalledWith('Copied to clipboard');
      });

      // Step 4: Close
      const closeButton = screen.getByRole('button', { name: /close detail view/i });
      fireEvent.click(closeButton);
      expect(mockOnClose).toHaveBeenCalled();

      // Step 5: Rerender as closed
      rerender(
        <EntryDetailView
          entry={mockEntry}
          isOpen={false}
          onClose={mockOnClose}
        />
      );

      expect(screen.queryByRole('complementary')).not.toBeInTheDocument();

      // Step 6: Reopen with different entry
      vi.clearAllMocks();
      mockCopyToClipboard.mockResolvedValue(true);

      rerender(
        <EntryDetailView
          entry={mockEntry2}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      // Step 7: Verify new entry data
      expect(screen.getByText('192.168.1.200')).toBeInTheDocument();
      expect(screen.getByText('def456')).toBeInTheDocument();

      // Step 8: Copy new entry
      const copyButton2 = screen.getByRole('button', { name: /copy full entry/i });
      fireEvent.click(copyButton2);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(mockEntry2.rawLine);
        expect(mockToastSuccess).toHaveBeenCalledWith('Copied to clipboard');
      });
    });

    it('should handle rapid open/close cycles', () => {
      const { rerender } = render(
        <EntryDetailView entry={mockEntry} isOpen={false} onClose={mockOnClose} />
      );

      // Open
      rerender(<EntryDetailView entry={mockEntry} isOpen={true} onClose={mockOnClose} />);
      expect(screen.getByRole('complementary')).toBeInTheDocument();

      // Close
      rerender(<EntryDetailView entry={mockEntry} isOpen={false} onClose={mockOnClose} />);
      expect(screen.queryByRole('complementary')).not.toBeInTheDocument();

      // Open again
      rerender(<EntryDetailView entry={mockEntry} isOpen={true} onClose={mockOnClose} />);
      expect(screen.getByRole('complementary')).toBeInTheDocument();

      // Close again
      rerender(<EntryDetailView entry={mockEntry} isOpen={false} onClose={mockOnClose} />);
      expect(screen.queryByRole('complementary')).not.toBeInTheDocument();
    });

    it('should handle entry switching while open', () => {
      const { rerender } = render(
        <EntryDetailView entry={mockEntry} isOpen={true} onClose={mockOnClose} />
      );

      // Verify first entry
      expect(screen.getByText('192.168.1.100')).toBeInTheDocument();
      expect(screen.getByText('10.0.0.1')).toBeInTheDocument();

      // Switch to second entry while still open
      rerender(<EntryDetailView entry={mockEntry2} isOpen={true} onClose={mockOnClose} />);

      // Verify second entry
      expect(screen.getByText('10.0.0.50')).toBeInTheDocument();
      expect(screen.getByText('192.168.1.200')).toBeInTheDocument();

      // Switch back to first entry
      rerender(<EntryDetailView entry={mockEntry} isOpen={true} onClose={mockOnClose} />);

      // Verify first entry again
      expect(screen.getByText('192.168.1.100')).toBeInTheDocument();
      expect(screen.getByText('10.0.0.1')).toBeInTheDocument();
    });
  });
});
