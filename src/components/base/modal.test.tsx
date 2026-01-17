import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../../test-utils';
import { Modal } from './modal';
import userEvent from '@testing-library/user-event';

describe('Modal', () => {
  it('does not render when isOpen is false', () => {
    render(
      <Modal isOpen={false} onClose={() => {}} title="Test Modal">
        Content
      </Modal>
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders when isOpen is true', () => {
    render(
      <Modal isOpen={true} onClose={() => {}} title="Test Modal">
        Content
      </Modal>
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Test Modal')).toBeInTheDocument();
    expect(screen.getByText('Content')).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', async () => {
    const handleClose = vi.fn();
    const user = userEvent.setup();
    render(
      <Modal isOpen={true} onClose={handleClose} title="Test">
        Content
      </Modal>
    );

    await user.click(screen.getByLabelText(/close modal/i));
    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when Escape key is pressed', async () => {
    const handleClose = vi.fn();
    const user = userEvent.setup();
    render(
      <Modal isOpen={true} onClose={handleClose} title="Test">
        Content
      </Modal>
    );

    await user.keyboard('{Escape}');
    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('applies size variants correctly', () => {
    const { rerender } = render(
      <Modal isOpen={true} onClose={() => {}} title="Test" size="sm">
        Content
      </Modal>
    );
    let dialog = screen.getByRole('dialog').querySelector('div');
    expect(dialog?.className).toContain('max-w-sm');

    rerender(
      <Modal isOpen={true} onClose={() => {}} title="Test" size="lg">
        Content
      </Modal>
    );
    dialog = screen.getByRole('dialog').querySelector('div');
    expect(dialog?.className).toContain('max-w-lg');
  });

  it('has proper ARIA attributes', () => {
    render(
      <Modal isOpen={true} onClose={() => {}} title="Accessible Modal">
        Content
      </Modal>
    );
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAttribute('aria-labelledby');
  });

  it('supports dark mode classes', () => {
    document.documentElement.classList.add('dark');
    render(
      <Modal isOpen={true} onClose={() => {}} title="Dark Mode">
        Content
      </Modal>
    );
    const dialog = screen.getByRole('dialog').querySelector('div');
    expect(dialog?.className).toContain('dark:bg-gray-800');
  });
});
