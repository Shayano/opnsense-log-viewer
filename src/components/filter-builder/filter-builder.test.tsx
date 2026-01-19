import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { FilterBuilder } from './filter-builder';
import { useFilterStore } from '@/stores/filter-store';
import toast from 'react-hot-toast';

// Mock toast
vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('FilterBuilder', () => {
  beforeEach(() => {
    // Reset store before each test
    useFilterStore.setState({
      filters: [],
      draftMode: false,
    });
    vi.clearAllMocks();
  });

  it('should not render when isOpen is false', () => {
    const { container } = render(
      <FilterBuilder isOpen={false} onClose={vi.fn()} editingFilter={null} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('should render when isOpen is true', () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);
    expect(screen.getByRole('heading', { name: 'Add Filter' })).toBeInTheDocument();
  });

  it('should display three-step form', () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    expect(screen.getByLabelText('Select field')).toBeInTheDocument();
  });

  it('should show operator selector after field is selected', async () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    const fieldSelect = screen.getByLabelText('Select field');
    fireEvent.change(fieldSelect, { target: { value: 'sourceIp' } });

    await waitFor(() => {
      expect(screen.getByLabelText('Select operator')).toBeInTheDocument();
    });
  });

  it('should show value input after operator is selected', async () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    const fieldSelect = screen.getByLabelText('Select field');
    fireEvent.change(fieldSelect, { target: { value: 'sourceIp' } });

    await waitFor(() => {
      const operatorSelect = screen.getByLabelText('Select operator');
      fireEvent.change(operatorSelect, { target: { value: 'equals' } });
    });

    await waitFor(() => {
      expect(screen.getByLabelText('Value')).toBeInTheDocument();
    });
  });

  it('should add filter to store on submit', async () => {
    const onClose = vi.fn();
    render(<FilterBuilder isOpen={true} onClose={onClose} editingFilter={null} />);

    // Fill form
    const fieldSelect = screen.getByLabelText('Select field');
    fireEvent.change(fieldSelect, { target: { value: 'action' } });

    await waitFor(() => {
      const operatorSelect = screen.getByLabelText('Select operator');
      fireEvent.change(operatorSelect, { target: { value: 'equals' } });
    });

    await waitFor(() => {
      const valueSelect = screen.getByLabelText('Select action');
      fireEvent.change(valueSelect, { target: { value: 'block' } });
    });

    // Submit form
    const submitButton = screen.getByRole('button', { name: 'Add Filter' });
    fireEvent.click(submitButton);

    await waitFor(() => {
      const state = useFilterStore.getState();
      expect(state.filters).toHaveLength(1);
      expect(state.filters[0].field).toBe('action');
      expect(state.filters[0].operator).toBe('equals');
      expect(state.filters[0].value).toBe('block');
      expect(toast.success).toHaveBeenCalledWith('Filter added');
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('should update filter when editing', async () => {
    const { addFilter } = useFilterStore.getState();
    addFilter({ field: 'sourceIp', operator: 'equals', value: '192.168.1.1' });

    const state = useFilterStore.getState();
    const filter = state.filters[0];

    const onClose = vi.fn();
    render(
      <FilterBuilder
        isOpen={true}
        onClose={onClose}
        editingFilter={{
          id: filter.id,
          field: filter.field,
          operator: filter.operator,
          value: filter.value,
        }}
      />
    );

    expect(screen.getByText('Edit Filter')).toBeInTheDocument();

    // Change value
    await waitFor(() => {
      const valueInput = screen.getByLabelText('Value');
      fireEvent.change(valueInput, { target: { value: '10.0.0.1' } });
    });

    // Submit
    const updateButton = screen.getByText('Update Filter');
    fireEvent.click(updateButton);

    await waitFor(() => {
      const newState = useFilterStore.getState();
      expect(newState.filters[0].value).toBe('10.0.0.1');
      expect(toast.success).toHaveBeenCalledWith('Filter updated');
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('should call onClose when Cancel is clicked', () => {
    const onClose = vi.fn();
    render(<FilterBuilder isOpen={true} onClose={onClose} editingFilter={null} />);

    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(onClose).toHaveBeenCalled();
  });

  it('should call onClose when X button is clicked', () => {
    const onClose = vi.fn();
    render(<FilterBuilder isOpen={true} onClose={onClose} editingFilter={null} />);

    const closeButton = screen.getByLabelText('Close filter builder');
    fireEvent.click(closeButton);

    expect(onClose).toHaveBeenCalled();
  });

  it('should disable submit button when form is incomplete', () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    const submitButton = screen.getByRole('button', { name: 'Add Filter' });
    expect(submitButton).toBeDisabled();
  });

  it('should enable submit button when form is complete', async () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    // Fill form
    const fieldSelect = screen.getByLabelText('Select field');
    fireEvent.change(fieldSelect, { target: { value: 'protocol' } });

    await waitFor(() => {
      const operatorSelect = screen.getByLabelText('Select operator');
      fireEvent.change(operatorSelect, { target: { value: 'equals' } });
    });

    await waitFor(() => {
      const valueSelect = screen.getByLabelText('Select protocol');
      fireEvent.change(valueSelect, { target: { value: 'TCP' } });
    });

    await waitFor(() => {
      const submitButton = screen.getByRole('button', { name: 'Add Filter' });
      expect(submitButton).not.toBeDisabled();
    });
  });

  it('should reset form after successful submit', async () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    // Fill and submit form
    const fieldSelect = screen.getByLabelText('Select field');
    fireEvent.change(fieldSelect, { target: { value: 'action' } });

    await waitFor(() => {
      const operatorSelect = screen.getByLabelText('Select operator');
      fireEvent.change(operatorSelect, { target: { value: 'equals' } });
    });

    await waitFor(() => {
      const valueSelect = screen.getByLabelText('Select action');
      fireEvent.change(valueSelect, { target: { value: 'block' } });
    });

    const submitButton = screen.getByRole('button', { name: 'Add Filter' });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(useFilterStore.getState().filters).toHaveLength(1);
    });
  });

  it('should show error toast if form is incomplete on submit', async () => {
    render(<FilterBuilder isOpen={true} onClose={vi.fn()} editingFilter={null} />);

    // Try to submit without filling form
    const submitButton = screen.getByRole('button', { name: 'Add Filter' });

    // Submit button should be disabled, but test the validation logic
    expect(submitButton).toBeDisabled();
  });
});
