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
    removeItem: (key: string) => {
      delete store[key];
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
    // Reset store before each test
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

  describe('Rendering', () => {
    it('should not render when isOpen is false', () => {
      render(<SaveFilterModal isOpen={false} onClose={mockOnClose} />);
      expect(screen.queryByRole('heading', { name: 'Save Filter' })).not.toBeInTheDocument();
    });

    it('should render when isOpen is true', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      expect(screen.getByRole('heading', { name: 'Save Filter' })).toBeInTheDocument();
      expect(screen.getByLabelText('Enter filter name:')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('e.g., Nightly Port 443 Blocks')).toBeInTheDocument();
    });

    it('should have autofocus on filter name input', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      expect(input).toHaveAttribute('autoFocus');
    });

    it('should display info text about saved filters', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      expect(
        screen.getByText(/Saved filters can be loaded instantly without reconstructing filters manually/)
      ).toBeInTheDocument();
    });

    it('should have Save Filter button disabled when name is empty', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });
      expect(saveButton).toBeDisabled();
    });

    it('should enable Save Filter button when name is entered', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: 'My Filter' } });
      expect(saveButton).not.toBeDisabled();
    });
  });

  describe('Filter Name Validation', () => {
    it('should show error when trying to save with empty name', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      // Input some text then delete it
      const input = screen.getByLabelText('Enter filter name:');
      fireEvent.change(input, { target: { value: 'test' } });
      fireEvent.change(input, { target: { value: '   ' } }); // Only spaces

      // Try to click save (button should be disabled, but test the validation logic)
      fireEvent.click(saveButton);

      // No save should occur
      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should show error for duplicate filter name (case-insensitive)', () => {
      // Add existing filter
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
        savedFilters: [
          {
            id: 'saved-1',
            name: 'Existing Filter',
            filters: [{ field: 'sourceIp', operator: 'equals', value: '10.0.0.1' }],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });

      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      // Try to save with same name (different case)
      fireEvent.change(input, { target: { value: 'existing filter' } });
      fireEvent.click(saveButton);

      expect(screen.getByText('A filter with this name already exists')).toBeInTheDocument();
      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should clear error when user types after validation error', () => {
      // Add existing filter
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
        savedFilters: [
          {
            id: 'saved-1',
            name: 'Existing Filter',
            filters: [{ field: 'sourceIp', operator: 'equals', value: '10.0.0.1' }],
            timestamp: Date.now(),
            version: 1,
          },
        ],
      });

      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      // Trigger duplicate name error
      fireEvent.change(input, { target: { value: 'existing filter' } });
      fireEvent.click(saveButton);
      expect(screen.getByText('A filter with this name already exists')).toBeInTheDocument();

      // Type something else - error should clear
      fireEvent.change(input, { target: { value: 'New Filter' } });
      expect(screen.queryByText('A filter with this name already exists')).not.toBeInTheDocument();
    });
  });

  describe('Save Filter', () => {
    it('should save filter with valid name', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: 'My Test Filter' } });
      fireEvent.click(saveButton);

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(state.savedFilters[0].name).toBe('My Test Filter');
      expect(state.savedFilters[0].filters).toHaveLength(1);
      expect(toast.success).toHaveBeenCalledWith('Filter "My Test Filter" saved');
      expect(mockOnClose).toHaveBeenCalled();
    });

    it('should trim filter name before saving', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: '  My Filter  ' } });
      fireEvent.click(saveButton);

      const state = useFilterStore.getState();
      expect(state.savedFilters[0].name).toBe('My Filter');
    });

    it('should strip runtime IDs when saving filters', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: 'Test Filter' } });
      fireEvent.click(saveButton);

      const state = useFilterStore.getState();
      // Saved filters should not have 'id' property in their filter objects
      expect(state.savedFilters[0].filters[0]).not.toHaveProperty('id');
      expect(state.savedFilters[0].filters[0].field).toBe('sourceIp');
    });

    it('should clear input and close modal after successful save', () => {
      const { rerender } = render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:') as HTMLInputElement;
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: 'Test Filter' } });
      fireEvent.click(saveButton);

      expect(mockOnClose).toHaveBeenCalled();

      // Reopen modal - input should be clear
      rerender(<SaveFilterModal isOpen={false} onClose={mockOnClose} />);
      rerender(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const newInput = screen.getByLabelText('Enter filter name:') as HTMLInputElement;
      expect(newInput.value).toBe('');
    });
  });

  describe('Keyboard Shortcuts', () => {
    it('should save filter when Enter key is pressed', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.change(input, { target: { value: 'Keyboard Filter' } });
      fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(1);
      expect(state.savedFilters[0].name).toBe('Keyboard Filter');
      expect(mockOnClose).toHaveBeenCalled();
    });

    it('should close modal when Escape key is pressed', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.keyDown(input, { key: 'Escape', code: 'Escape' });

      expect(mockOnClose).toHaveBeenCalled();
      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(0); // No save occurred
    });

    it('should not save when Enter is pressed with empty name', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');

      fireEvent.change(input, { target: { value: '' } });
      fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });

      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(0);
      expect(mockOnClose).not.toHaveBeenCalled();
    });
  });

  describe('Cancel Button', () => {
    it('should close modal when Cancel button is clicked', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const cancelButton = screen.getByRole('button', { name: /Cancel/i });

      fireEvent.click(cancelButton);

      expect(mockOnClose).toHaveBeenCalled();
      const state = useFilterStore.getState();
      expect(state.savedFilters).toHaveLength(0); // No save occurred
    });

    it('should clear input when closing via Cancel', () => {
      const { rerender } = render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const cancelButton = screen.getByRole('button', { name: /Cancel/i });

      fireEvent.change(input, { target: { value: 'Some Text' } });
      fireEvent.click(cancelButton);

      // Reopen modal - input should be clear
      rerender(<SaveFilterModal isOpen={false} onClose={mockOnClose} />);
      rerender(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const newInput = screen.getByLabelText('Enter filter name:') as HTMLInputElement;
      expect(newInput.value).toBe('');
    });
  });

  describe('Close Button (X)', () => {
    it('should close modal when X button is clicked', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const closeButton = screen.getByLabelText('Close save filter dialog');

      fireEvent.click(closeButton);

      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should show error toast on QuotaExceededError', () => {
      // Mock saveFilter to throw QuotaExceededError
      const saveFilterSpy = vi.spyOn(useFilterStore.getState(), 'saveFilter');
      const quotaError = new DOMException('QuotaExceededError', 'QuotaExceededError');
      quotaError.name = 'QuotaExceededError';
      saveFilterSpy.mockImplementation(() => {
        throw quotaError;
      });

      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: 'Test Filter' } });
      fireEvent.click(saveButton);

      expect(toast.error).toHaveBeenCalledWith('Storage full. Delete old filters to make room.');
      expect(mockOnClose).not.toHaveBeenCalled(); // Modal stays open on error

      saveFilterSpy.mockRestore();
    });

    it('should show generic error toast on other errors', () => {
      // Mock saveFilter to throw generic error
      const saveFilterSpy = vi.spyOn(useFilterStore.getState(), 'saveFilter');
      saveFilterSpy.mockImplementation(() => {
        throw new Error('Some other error');
      });

      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);
      const input = screen.getByLabelText('Enter filter name:');
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      fireEvent.change(input, { target: { value: 'Test Filter' } });
      fireEvent.click(saveButton);

      expect(toast.error).toHaveBeenCalledWith('Failed to save filter');
      expect(mockOnClose).not.toHaveBeenCalled(); // Modal stays open on error

      saveFilterSpy.mockRestore();
    });
  });

  describe('Accessibility', () => {
    it('should have proper ARIA labels', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);

      expect(screen.getByLabelText('Enter filter name:')).toBeInTheDocument();
      expect(screen.getByLabelText('Close save filter dialog')).toBeInTheDocument();
    });

    it('should have proper button types', () => {
      render(<SaveFilterModal isOpen={true} onClose={mockOnClose} />);

      const cancelButton = screen.getByRole('button', { name: /Cancel/i });
      const saveButton = screen.getByRole('button', { name: /Save Filter/i });

      expect(cancelButton).toHaveAttribute('type', 'button');
      expect(saveButton).toHaveAttribute('type', 'button');
    });
  });
});
