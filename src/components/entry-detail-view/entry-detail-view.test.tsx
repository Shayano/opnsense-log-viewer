import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { EntryDetailView } from './entry-detail-view';
import { Action, Protocol, type LogEntry } from '@/types/log-entry';
import * as clipboard from '@/utils/clipboard';
import toast from 'react-hot-toast';

vi.mock('@/utils/clipboard');
vi.mock('react-hot-toast');

describe('EntryDetailView', () => {
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

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('should not render when isOpen is false', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={false}
          onClose={mockOnClose}
        />
      );

      expect(
        screen.queryByRole('complementary', { name: /log entry details/i })
      ).not.toBeInTheDocument();
    });

    it('should not render when entry is null', () => {
      render(
        <EntryDetailView entry={null} isOpen={true} onClose={mockOnClose} />
      );

      expect(
        screen.queryByRole('complementary', { name: /log entry details/i })
      ).not.toBeInTheDocument();
    });

    it('should render detail view when open and entry exists', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(
        screen.getByRole('complementary', { name: /log entry details/i })
      ).toBeInTheDocument();
      expect(screen.getByText('Entry Details')).toBeInTheDocument();
    });

    it('should render raw log line section', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(screen.getByText('Raw Log Line')).toBeInTheDocument();
      expect(screen.getByDisplayValue(mockEntry.rawLine!)).toBeInTheDocument();
    });

    it('should render parsed fields section', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(screen.getByText('Parsed Fields')).toBeInTheDocument();
      expect(screen.getByText('Timestamp:')).toBeInTheDocument();
      expect(screen.getByText('Interface:')).toBeInTheDocument();
      expect(screen.getByText('Source IP:')).toBeInTheDocument();
      expect(screen.getByText('Source Port:')).toBeInTheDocument();
      expect(screen.getByText('Destination IP:')).toBeInTheDocument();
      expect(screen.getByText('Destination Port:')).toBeInTheDocument();
      expect(screen.getByText('Protocol:')).toBeInTheDocument();
      expect(screen.getByText('Action:')).toBeInTheDocument();
      expect(screen.getByText('Rule Label:')).toBeInTheDocument();
    });

    it('should display all field values correctly', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(screen.getByText(mockEntry.timestamp)).toBeInTheDocument();
      expect(screen.getByText(mockEntry.interface)).toBeInTheDocument();
      expect(screen.getByText(mockEntry.sourceIp)).toBeInTheDocument();
      expect(screen.getByText('443')).toBeInTheDocument();
      expect(screen.getByText(mockEntry.destinationIp)).toBeInTheDocument();
      expect(screen.getByText('80')).toBeInTheDocument();
      expect(screen.getByText('TCP')).toBeInTheDocument();
      expect(screen.getByText('BLOCK')).toBeInTheDocument();
      expect(screen.getByText(mockEntry.ruleLabel)).toBeInTheDocument();
    });

    it('should handle missing rawLine gracefully', () => {
      const entryWithoutRaw = { ...mockEntry, rawLine: undefined };

      render(
        <EntryDetailView
          entry={entryWithoutRaw}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(
        screen.getByDisplayValue('Raw log line not available')
      ).toBeInTheDocument();
    });
  });

  describe('enrichment data', () => {
    it('should display enriched interface name when available', () => {
      const enrichmentData = {
        interfaceNames: new Map([['vtnet0', 'LAN']]),
      };

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          enrichmentData={enrichmentData}
        />
      );

      expect(screen.getByText('LAN (vtnet0)')).toBeInTheDocument();
    });

    it('should display enriched rule label when available', () => {
      const enrichmentData = {
        ruleLabels: new Map([['abc123', 'Block RFC1918 Networks']]),
      };

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          enrichmentData={enrichmentData}
        />
      );

      expect(
        screen.getByText('Block RFC1918 Networks (abc123)')
      ).toBeInTheDocument();
    });

    it('should display both enriched values when available', () => {
      const enrichmentData = {
        interfaceNames: new Map([['vtnet0', 'LAN']]),
        ruleLabels: new Map([['abc123', 'Block RFC1918 Networks']]),
      };

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          enrichmentData={enrichmentData}
        />
      );

      expect(screen.getByText('LAN (vtnet0)')).toBeInTheDocument();
      expect(
        screen.getByText('Block RFC1918 Networks (abc123)')
      ).toBeInTheDocument();
    });

    it('should fallback to raw values when enrichment not available', () => {
      const enrichmentData = {
        interfaceNames: new Map(),
        ruleLabels: new Map(),
      };

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          enrichmentData={enrichmentData}
        />
      );

      expect(screen.getByText('vtnet0')).toBeInTheDocument();
      expect(screen.getByText('abc123')).toBeInTheDocument();
    });
  });

  describe('copy functionality', () => {
    it('should copy full entry to clipboard when copy button clicked', async () => {
      mockCopyToClipboard.mockResolvedValue(true);

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const copyButton = screen.getByRole('button', {
        name: /copy full entry/i,
      });
      fireEvent.click(copyButton);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(mockEntry.rawLine);
        expect(mockToastSuccess).toHaveBeenCalledWith(
          'Copied to clipboard'
        );
      });
    });

    it('should show error toast when copy fails', async () => {
      mockCopyToClipboard.mockResolvedValue(false);

      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const copyButton = screen.getByRole('button', {
        name: /copy full entry/i,
      });
      fireEvent.click(copyButton);

      await waitFor(() => {
        expect(mockToastError).toHaveBeenCalledWith(
          'Failed to copy to clipboard'
        );
      });
    });

    it('should copy empty string when rawLine is missing', async () => {
      const entryWithoutRaw = { ...mockEntry, rawLine: undefined };
      mockCopyToClipboard.mockResolvedValue(true);

      render(
        <EntryDetailView
          entry={entryWithoutRaw}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const copyButton = screen.getByRole('button', {
        name: /copy full entry/i,
      });
      fireEvent.click(copyButton);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith('');
      });
    });
  });

  describe('close functionality', () => {
    it('should call onClose when close button clicked', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const closeButton = screen.getByRole('button', {
        name: /close detail view/i,
      });
      fireEvent.click(closeButton);

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('should call onClose when Escape key pressed', () => {
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

    it('should not call onClose when other keys pressed', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      fireEvent.keyDown(document, { key: 'Enter' });
      fireEvent.keyDown(document, { key: 'a' });

      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should call onClose when clicking outside the detail pane', () => {
      render(
        <div data-testid="outside">
          <EntryDetailView
            entry={mockEntry}
            isOpen={true}
            onClose={mockOnClose}
          />
        </div>
      );

      const outside = screen.getByTestId('outside');
      fireEvent.mouseDown(outside);

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('should not call onClose when clicking inside the detail pane', () => {
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

  describe('accessibility', () => {
    it('should have proper ARIA role', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(
        screen.getByRole('complementary', { name: /log entry details/i })
      ).toBeInTheDocument();
    });

    it('should have proper ARIA label on close button', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(
        screen.getByRole('button', { name: /close detail view/i })
      ).toBeInTheDocument();
    });

    it('should have proper ARIA label on textarea', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(screen.getByLabelText(/raw log entry/i)).toBeInTheDocument();
    });

    it('should support keyboard navigation to close button', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const closeButton = screen.getByRole('button', {
        name: /close detail view/i,
      });
      closeButton.focus();
      expect(closeButton).toHaveFocus();
    });
  });

  describe('layout and styling', () => {
    it('should apply correct height class when open', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const detailView = screen.getByRole('complementary');
      expect(detailView).toHaveClass('h-[40vh]');
    });

    it('should have split layout grid', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const contentArea = screen
        .getByRole('complementary')
        .querySelector('.grid-cols-2');
      expect(contentArea).toBeInTheDocument();
    });

    it('should have monospace font for raw log textarea', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const textarea = screen.getByLabelText(/raw log entry/i);
      expect(textarea).toHaveClass('font-mono');
    });

    it('should have read-only textarea', () => {
      render(
        <EntryDetailView
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const textarea = screen.getByLabelText(/raw log entry/i);
      expect(textarea).toHaveAttribute('readonly');
    });
  });

  describe('edge cases', () => {
    it('should handle entry with zero port numbers', () => {
      const entryWithZeroPorts = {
        ...mockEntry,
        sourcePort: 0,
        destinationPort: 0,
      };

      render(
        <EntryDetailView
          entry={entryWithZeroPorts}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      expect(screen.getAllByText('0')).toHaveLength(2);
    });

    it('should handle entry with different protocols', () => {
      const udpEntry = { ...mockEntry, protocol: Protocol.UDP };

      render(
        <EntryDetailView entry={udpEntry} isOpen={true} onClose={mockOnClose} />
      );

      expect(screen.getByText('UDP')).toBeInTheDocument();
    });

    it('should handle entry with different actions', () => {
      const passEntry = { ...mockEntry, action: Action.PASS };

      render(
        <EntryDetailView entry={passEntry} isOpen={true} onClose={mockOnClose} />
      );

      expect(screen.getByText('PASS')).toBeInTheDocument();
    });

    it('should handle very long field values', () => {
      const longEntry = {
        ...mockEntry,
        ruleLabel: 'A'.repeat(200),
      };

      render(
        <EntryDetailView
          entry={longEntry}
          isOpen={true}
          onClose={mockOnClose}
        />
      );

      const ruleLabel = screen.getByText(longEntry.ruleLabel);
      expect(ruleLabel).toBeInTheDocument();
      expect(ruleLabel).toHaveClass('truncate');
    });
  });
});
