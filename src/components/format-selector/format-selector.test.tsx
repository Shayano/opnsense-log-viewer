import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { FormatSelector } from './format-selector';

describe('FormatSelector', () => {
  it('renders when open', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(<FormatSelector isOpen={true} onConfirm={onConfirm} onCancel={onCancel} />);

    expect(screen.getByText('Select Log Format')).toBeInTheDocument();
    expect(screen.getByText('RFC3164 (Legacy Syslog)')).toBeInTheDocument();
    expect(screen.getByText('RFC5424 (Modern Syslog)')).toBeInTheDocument();
    expect(screen.getByText('CSV filterlog (OPNsense)')).toBeInTheDocument();
  });

  it('does not render when closed', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(<FormatSelector isOpen={false} onConfirm={onConfirm} onCancel={onCancel} />);

    expect(screen.queryByText('Select Log Format')).not.toBeInTheDocument();
  });

  it('calls onConfirm with selected format when confirmed', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(<FormatSelector isOpen={true} onConfirm={onConfirm} onCancel={onCancel} />);

    // Select RFC5424
    const rfc5424Radio = screen.getByLabelText(/RFC5424 \(Modern Syslog\)/i, { selector: 'input' });
    fireEvent.click(rfc5424Radio);

    // Click Confirm
    const confirmButton = screen.getByRole('button', { name: /confirm/i });
    fireEvent.click(confirmButton);

    expect(onConfirm).toHaveBeenCalledWith('RFC5424');
  });

  it('calls onCancel when cancel button clicked', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(<FormatSelector isOpen={true} onConfirm={onConfirm} onCancel={onCancel} />);

    const cancelButton = screen.getByRole('button', { name: /cancel/i });
    fireEvent.click(cancelButton);

    expect(onCancel).toHaveBeenCalled();
  });

  it('defaults to RFC3164 format', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(<FormatSelector isOpen={true} onConfirm={onConfirm} onCancel={onCancel} />);

    const rfc3164Radio = screen.getByLabelText(/RFC3164 \(Legacy Syslog\)/i, { selector: 'input' });
    expect(rfc3164Radio).toBeChecked();
  });

  it('allows changing format selection', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(<FormatSelector isOpen={true} onConfirm={onConfirm} onCancel={onCancel} />);

    // Initially RFC3164 is selected
    const rfc3164Radio = screen.getByLabelText(/RFC3164 \(Legacy Syslog\)/i, { selector: 'input' });
    expect(rfc3164Radio).toBeChecked();

    // Select CSV
    const csvRadio = screen.getByLabelText(/CSV filterlog \(OPNsense\)/i, { selector: 'input' });
    fireEvent.click(csvRadio);

    expect(csvRadio).toBeChecked();
    expect(rfc3164Radio).not.toBeChecked();
  });
});
