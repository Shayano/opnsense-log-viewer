import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { CopyButton } from './copy-button';
import * as clipboard from '@/utils/clipboard';
import toast from 'react-hot-toast';

vi.mock('@/utils/clipboard');
vi.mock('react-hot-toast');

describe('CopyButton', () => {
  const mockCopyToClipboard = vi.mocked(clipboard.copyToClipboard);
  const mockToastSuccess = vi.mocked(toast.success);
  const mockToastError = vi.mocked(toast.error);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('compact mode', () => {
    it('should render compact button with copy icon', () => {
      render(<CopyButton value="test value" compact />);

      const button = screen.getByRole('button', { name: /copy value/i });
      expect(button).toBeInTheDocument();
      expect(button).toHaveClass('p-1');
    });

    it('should copy value to clipboard when clicked', async () => {
      mockCopyToClipboard.mockResolvedValue(true);

      render(<CopyButton value="192.168.1.1" compact />);

      const button = screen.getByRole('button', { name: /copy value/i });
      fireEvent.click(button);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith('192.168.1.1');
      });
    });

    it('should use custom label in aria-label', () => {
      render(<CopyButton value="test" label="IP address" compact />);

      const button = screen.getByRole('button', { name: /copy ip address/i });
      expect(button).toBeInTheDocument();
    });
  });

  describe('normal mode', () => {
    it('should render button with label text', () => {
      render(<CopyButton value="test value" label="Copy Entry" />);

      const button = screen.getByRole('button', { name: /copy entry/i });
      expect(button).toBeInTheDocument();
      expect(screen.getByText('Copy Entry')).toBeInTheDocument();
    });

    it('should use default label when not provided', () => {
      render(<CopyButton value="test value" />);

      const button = screen.getByRole('button', { name: /copy value/i });
      expect(button).toBeInTheDocument();
      expect(screen.getByText('Copy')).toBeInTheDocument();
    });

    it('should copy value to clipboard when clicked', async () => {
      mockCopyToClipboard.mockResolvedValue(true);

      render(<CopyButton value="Full log entry text" label="Copy Full Entry" />);

      const button = screen.getByRole('button', { name: /copy full entry/i });
      fireEvent.click(button);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith('Full log entry text');
      });
    });

    it('should handle keyboard navigation (focus)', () => {
      render(<CopyButton value="test" label="Copy" />);

      const button = screen.getByRole('button', { name: /copy/i });
      button.focus();
      expect(button).toHaveFocus();
    });

    it('should be keyboard accessible (native button behavior)', () => {
      render(<CopyButton value="test value" label="Copy" />);

      const button = screen.getByRole('button', { name: /copy/i });
      expect(button.tagName).toBe('BUTTON');
      // Native buttons support Enter/Space activation automatically
    });
  });

  describe('edge cases', () => {
    it('should handle empty string value', async () => {
      mockCopyToClipboard.mockResolvedValue(true);

      render(<CopyButton value="" compact />);

      const button = screen.getByRole('button', { name: /copy value/i });
      fireEvent.click(button);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith('');
      });
    });

    it('should handle very long values', async () => {
      const longValue = 'A'.repeat(10000);
      mockCopyToClipboard.mockResolvedValue(true);

      render(<CopyButton value={longValue} compact />);

      const button = screen.getByRole('button', { name: /copy value/i });
      fireEvent.click(button);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(longValue);
      });
    });

    it('should handle special characters in value', async () => {
      const specialValue = 'Test: ñ 中文 🔥 \n\t\r';
      mockCopyToClipboard.mockResolvedValue(true);

      render(<CopyButton value={specialValue} compact />);

      const button = screen.getByRole('button', { name: /copy value/i });
      fireEvent.click(button);

      await waitFor(() => {
        expect(mockCopyToClipboard).toHaveBeenCalledWith(specialValue);
      });
    });
  });
});
