import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { DeleteConfirmDialog } from './delete-confirm-dialog';

describe('DeleteConfirmDialog', () => {
  const mockOnConfirm = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should not render when closed', () => {
    render(
      <DeleteConfirmDialog
        isOpen={false}
        filterName="Test Filter"
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );
    expect(screen.queryByRole('heading', { name: 'Delete Filter' })).not.toBeInTheDocument();
  });

  it('should render when open with filter name', () => {
    render(
      <DeleteConfirmDialog
        isOpen={true}
        filterName="My Test Filter"
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );
    expect(screen.getByRole('heading', { name: 'Delete Filter' })).toBeInTheDocument();
    expect(screen.getByText('"My Test Filter"')).toBeInTheDocument();
    expect(screen.getByText('⚠️ This cannot be undone.')).toBeInTheDocument();
  });

  it('should call onConfirm when Delete button clicked', () => {
    render(
      <DeleteConfirmDialog
        isOpen={true}
        filterName="Test Filter"
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );

    const buttons = screen.getAllByRole('button');
    const deleteButton = buttons.find(btn => btn.textContent?.includes('Delete') && !btn.getAttribute('aria-label'));
    if (!deleteButton) throw new Error('Delete button not found');

    fireEvent.click(deleteButton);

    expect(mockOnConfirm).toHaveBeenCalled();
    expect(mockOnCancel).not.toHaveBeenCalled();
  });

  it('should call onCancel when Cancel button clicked', () => {
    render(
      <DeleteConfirmDialog
        isOpen={true}
        filterName="Test Filter"
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
      <DeleteConfirmDialog
        isOpen={true}
        filterName="Test Filter"
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
      <DeleteConfirmDialog
        isOpen={true}
        filterName="Test Filter"
        onConfirm={mockOnConfirm}
        onCancel={mockOnCancel}
      />
    );

    const dialog = container.firstChild as HTMLElement;
    fireEvent.keyDown(dialog, { key: 'Escape' });

    expect(mockOnCancel).toHaveBeenCalled();
  });
});
