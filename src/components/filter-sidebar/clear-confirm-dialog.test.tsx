import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ClearFiltersConfirmDialog } from './clear-confirm-dialog';

describe('ClearFiltersConfirmDialog', () => {
  const mockOnConfirm = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should not render when closed', () => {
    render(
      <ClearFiltersConfirmDialog
        isOpen={false}
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );
    expect(screen.queryByRole('heading', { name: 'Clear All Filters' })).not.toBeInTheDocument();
  });

  it('should render when open', () => {
    render(
      <ClearFiltersConfirmDialog
        isOpen={true}
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );
    expect(screen.getByRole('heading', { name: 'Clear All Filters' })).toBeInTheDocument();
    expect(screen.getByText('Clear all active filters?')).toBeInTheDocument();
  });

  it('should call onConfirm when Clear All button clicked', () => {
    render(
      <ClearFiltersConfirmDialog
        isOpen={true}
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );

    const clearButton = screen.getByRole('button', { name: /Clear All/i });
    fireEvent.click(clearButton);

    expect(mockOnConfirm).toHaveBeenCalled();
    expect(mockOnCancel).not.toHaveBeenCalled();
  });

  it('should call onCancel when Cancel button clicked', () => {
    render(
      <ClearFiltersConfirmDialog
        isOpen={true}
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );

    const cancelButton = screen.getByRole('button', { name: /Cancel/i });
    fireEvent.click(cancelButton);

    expect(mockOnCancel).toHaveBeenCalled();
    expect(mockOnConfirm).not.toHaveBeenCalled();
  });

  it('should call onConfirm on Enter key', () => {
    const { container } = render(
      <ClearFiltersConfirmDialog
        isOpen={true}
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );

    const dialog = container.firstChild as HTMLElement;
    fireEvent.keyDown(dialog, { key: 'Enter' });

    expect(mockOnConfirm).toHaveBeenCalled();
  });

  it('should call onCancel on Escape key', () => {
    const { container } = render(
      <ClearFiltersConfirmDialog
        isOpen={true}
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );

    const dialog = container.firstChild as HTMLElement;
    fireEvent.keyDown(dialog, { key: 'Escape' });

    expect(mockOnCancel).toHaveBeenCalled();
  });
});
