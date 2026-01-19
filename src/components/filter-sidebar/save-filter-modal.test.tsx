import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SaveFilterModal } from './save-filter-modal';
import { useFilterStore } from '@/stores/filter-store';
import toast from 'react-hot-toast';

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    clear: () => {
      store = {};
    },
  };
})();
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

describe('SaveFilterModal', () => {
  const mockOnClose = vi.fn();

  beforeEach(() => {
    useFilterStore.setState({
      filters: [
        {
          id: '1',
          field: 'sourceIp',
          operator: 'equals',
          value: '192.168.1.1',
          logic: 'AND',
        },
      ],
      draftMode: true,
      savedFilters: [],
    });
    localStorageMock.clear();
    vi.clearAllMocks();
    mockOnClose.mockClear();
  });

  describe('Basic Rendering', () => {
    it('should not render when closed', () => {
      render(<SaveFilterModal isOpen={false} onClose={mockOnClose} />);
      expect(screen.queryByRole('heading', { name: 'Save Filter' })).not.toBeInTheDocument();
    });

    it('should render when open', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      expect(screen.getByRole('heading', { name: 'Save Filter' })).toBeInTheDocument();
      expect(screen.getByLabelText('Enter filter name:')).toBeInTheDocument();
    });
  });

  describe('Save Functionality', () => {
    it('should save filter with valid name', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.change(input, { target: { value: 'Test Filter' } });

      // Find Save button by finding all buttons and picking the one with Save icon
      const buttons = screen.getAllByRole('button');
      const saveButton = buttons.find(btn => btn.textContent?.includes('Save Filter'));
      if (!saveButton) throw new Error('Save button not found');

      fireEvent.click(saveButton);

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(state.savedFilters[0].name).toBe('Test Filter');
      expect(toast.success).toHaveBeenCalledWith('Filter "Test Filter" saved');
    });

    it('should reject duplicate names (case-insensitive)', () => {
      useFilterStore.setState({
        filters: [{ id: '1', field: 'sourceIp', operator: 'equals', value: '192.168.1.1', logic: 'AND' }],
        draftMode: true,
        savedFilters: [{
          id: 'existing',
          name: 'Existing Filter',
          filters: [{ field: 'sourceIp', operator: 'equals', value: '10.0.0.1' }],
          timestamp: Date.now(),
          version: 1,
        }],
      });

      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.change(input, { target: { value: 'existing filter' } });

      const buttons = screen.getAllByRole('button');
      const saveButton = buttons.find(btn => btn.textContent?.includes('Save Filter'));
      if (!saveButton) throw new Error('Save button not found');

      fireEvent.click(saveButton);

      expect(screen.getByText('A filter with this name already exists')).toBeInTheDocument();
      expect(mockOnClose).not.toHaveBeenCalled();
    });
  });

  describe('Keyboard Shortcuts', () => {
    it('should save on Enter key', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.change(input, { target: { value: 'Keyboard Test' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(state.savedFilters[0].name).toBe('Keyboard Test');
    });

    it('should close on Escape key', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.keyDown(input, { key: 'Escape' });

      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  describe('Cancel Button', () => {
    it('should close modal on cancel', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const cancelButton = screen.getByRole('button', { name: /Cancel/i });

      fireEvent.click(cancelButton);

      expect(mockOnClose).toHaveBeenCalled();
      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(0);
    });
  });
});
