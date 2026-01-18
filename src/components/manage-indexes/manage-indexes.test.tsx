import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ManageIndexes } from './manage-indexes';
import type { IndexInfo } from '@/types/file';

// Mock Tauri API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Mock toast
vi.mock('@/components/base/toaster', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('ManageIndexes', () => {
  const mockIndexes: IndexInfo[] = [
    {
      hash: 'abc123',
      sourceFilePath: '/path/to/file1.log',
      sourceFileExists: true,
      createdAt: '2026-01-18T10:00:00Z',
      fileSize: 1024 * 1024 * 100, // 100 MB
      entryCount: 50000,
      indexFileSize: 1024 * 1024 * 5, // 5 MB
    },
    {
      hash: 'def456',
      sourceFilePath: '/path/to/file2.log',
      sourceFileExists: false,
      createdAt: '2026-01-17T15:30:00Z',
      fileSize: 1024 * 1024 * 1024 * 2, // 2 GB
      entryCount: 1000000,
      indexFileSize: 1024 * 1024 * 100, // 100 MB
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render loading state initially', () => {
    mockInvoke.mockReturnValue(new Promise(() => {})); // Never resolves
    render(<ManageIndexes />);
    expect(screen.getByText('Loading indexes...')).toBeInTheDocument();
  });

  it('should display indexes when loaded', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('file1.log')).toBeInTheDocument();
      expect(screen.getByText('file2.log')).toBeInTheDocument();
    });
  });

  it('should display empty state when no indexes exist', async () => {
    mockInvoke.mockResolvedValue([]);
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('No saved indexes found')).toBeInTheDocument();
    });
  });

  it('should show check icon for existing source files', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('file1.log')).toBeInTheDocument();
    });

    // Check that the green check icon class exists (lucide icons don't have data-testid)
    const container = screen.getByText('file1.log').closest('div');
    expect(container?.querySelector('.text-green-500')).toBeInTheDocument();
  });

  it('should show alert icon for missing source files', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('⚠ Source file not found')).toBeInTheDocument();
    });
  });

  it('should format file sizes correctly', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    render(<ManageIndexes />);

    await waitFor(() => {
      // Multiple instances of 100.0 MB exist (both source and index sizes)
      expect(screen.getAllByText(/100\.0 MB/).length).toBeGreaterThan(0);
      expect(screen.getByText(/5\.0 MB/)).toBeInTheDocument(); // Index file
      expect(screen.getByText(/2\.00 GB/)).toBeInTheDocument(); // 2 GB file
    });
  });

  it('should format entry counts with locale formatting', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    render(<ManageIndexes />);

    await waitFor(() => {
      // Check for entry counts - locale formatting may vary
      expect(screen.getByText(/50[,\s]000 entries/)).toBeInTheDocument();
      expect(screen.getByText(/1[,\s]000[,\s]000 entries/)).toBeInTheDocument();
    });
  });

  it('should open delete confirmation modal when delete button clicked', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    const user = userEvent.setup();
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('file1.log')).toBeInTheDocument();
    });

    const deleteButtons = screen.getAllByLabelText('Delete index');
    await user.click(deleteButtons[0]);

    expect(screen.getByText('Delete Index?')).toBeInTheDocument();
    expect(
      screen.getByText('This will delete the index file. The source log file will not be affected.')
    ).toBeInTheDocument();
  });

  it('should delete index when confirmed', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    const user = userEvent.setup();
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('file1.log')).toBeInTheDocument();
    });

    // Click delete button
    const deleteButtons = screen.getAllByLabelText('Delete index');
    await user.click(deleteButtons[0]);

    // Confirm deletion
    mockInvoke.mockResolvedValueOnce(undefined); // delete_index_by_hash
    mockInvoke.mockResolvedValueOnce(mockIndexes.slice(1)); // Refresh list
    const confirmButton = screen.getByText('Delete');
    await user.click(confirmButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('delete_index_by_hash', { hash: 'abc123' });
    });
  });

  it('should cancel deletion when cancel button clicked', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    const user = userEvent.setup();
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('file1.log')).toBeInTheDocument();
    });

    // Click delete button
    const deleteButtons = screen.getAllByLabelText('Delete index');
    await user.click(deleteButtons[0]);

    // Click cancel
    const cancelButton = screen.getByText('Cancel');
    await user.click(cancelButton);

    // Modal should close
    await waitFor(() => {
      expect(screen.queryByText('Delete Index?')).not.toBeInTheDocument();
    });
  });

  it('should refresh indexes when refresh button clicked', async () => {
    mockInvoke.mockResolvedValue(mockIndexes);
    const user = userEvent.setup();
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('file1.log')).toBeInTheDocument();
    });

    mockInvoke.mockClear();
    mockInvoke.mockResolvedValue(mockIndexes);

    const refreshButton = screen.getByText('Refresh');
    await user.click(refreshButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('list_all_indexes');
    });
  });

  it('should handle API errors gracefully', async () => {
    mockInvoke.mockRejectedValue(new Error('Backend error'));
    render(<ManageIndexes />);

    await waitFor(() => {
      expect(screen.getByText('No saved indexes found')).toBeInTheDocument();
    });
  });
});
