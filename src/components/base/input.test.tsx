import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../../test-utils';
import { Input } from './input';
import userEvent from '@testing-library/user-event';

describe('Input', () => {
  it('renders with label', () => {
    render(<Input label="Username" />);
    expect(screen.getByLabelText(/username/i)).toBeInTheDocument();
  });

  it('renders without label', () => {
    render(<Input placeholder="Enter text" />);
    expect(screen.getByPlaceholderText(/enter text/i)).toBeInTheDocument();
  });

  it('handles text input', async () => {
    const handleChange = vi.fn();
    const user = userEvent.setup();
    render(<Input label="Name" onChange={handleChange} />);

    const input = screen.getByLabelText(/name/i);
    await user.type(input, 'Hello');

    expect(handleChange).toHaveBeenCalled();
    expect(input).toHaveValue('Hello');
  });

  it('displays error message', () => {
    render(<Input label="Email" error="Invalid email" />);
    expect(screen.getByRole('alert')).toHaveTextContent('Invalid email');
  });

  it('applies error styles when error prop is present', () => {
    render(<Input label="Email" error="Invalid" />);
    const input = screen.getByLabelText(/email/i);
    expect(input.className).toContain('border-error-500');
    expect(input).toHaveAttribute('aria-invalid', 'true');
  });

  it('applies disabled state correctly', () => {
    render(<Input label="Disabled" disabled />);
    const input = screen.getByLabelText(/disabled/i);
    expect(input).toBeDisabled();
    expect(input.className).toContain('opacity-50');
  });

  it('supports number type', () => {
    render(<Input type="number" label="Age" />);
    const input = screen.getByLabelText(/age/i);
    expect(input).toHaveAttribute('type', 'number');
  });

  it('has proper ARIA attributes', () => {
    render(<Input label="Test" error="Error message" />);
    const input = screen.getByLabelText(/test/i);
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input).toHaveAttribute('aria-describedby');
  });

  it('generates unique IDs for accessibility', () => {
    render(
      <>
        <Input label="First" />
        <Input label="Second" />
      </>
    );

    const first = screen.getByLabelText(/first/i);
    const second = screen.getByLabelText(/second/i);

    expect(first.id).not.toBe(second.id);
  });
});
