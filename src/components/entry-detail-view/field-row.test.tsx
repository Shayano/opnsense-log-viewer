import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FieldRow } from './field-row';
import * as clipboard from '@/utils/clipboard';
import toast from 'react-hot-toast';

vi.mock('@/utils/clipboard');
vi.mock('react-hot-toast');

describe('FieldRow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('should render label and value', () => {
      render(<FieldRow label="Source IP" value="192.168.1.100" />);

      expect(screen.getByText('Source IP:')).toBeInTheDocument();
      expect(screen.getByText('192.168.1.100')).toBeInTheDocument();
    });

    it('should render copy button', () => {
      render(<FieldRow label="Destination IP" value="10.0.0.1" />);

      const copyButton = screen.getByRole('button', { name: /copy value/i });
      expect(copyButton).toBeInTheDocument();
    });

    it('should apply monospace font to value', () => {
      render(<FieldRow label="Port" value="443" />);

      const valueElement = screen.getByText('443');
      expect(valueElement).toHaveClass('font-mono');
    });

    it('should apply truncate class to value for overflow', () => {
      render(<FieldRow label="Long Field" value="Very long value that should truncate" />);

      const valueElement = screen.getByText('Very long value that should truncate');
      expect(valueElement).toHaveClass('truncate');
    });

    it('should have title attribute on value for tooltip', () => {
      const testValue = 'Test tooltip value';
      render(<FieldRow label="Field" value={testValue} />);

      const valueElement = screen.getByText(testValue);
      expect(valueElement).toHaveAttribute('title', testValue);
    });
  });

  describe('layout and styling', () => {
    it('should have proper spacing and layout classes', () => {
      const { container } = render(<FieldRow label="Test" value="Value" />);

      const wrapper = container.firstChild;
      expect(wrapper).toHaveClass('flex', 'items-center', 'justify-between');
    });

    it('should apply hover effect class', () => {
      const { container } = render(<FieldRow label="Test" value="Value" />);

      const wrapper = container.firstChild;
      expect(wrapper).toHaveClass('hover:bg-gray-50', 'dark:hover:bg-gray-800');
    });

    it('should have fixed width label', () => {
      render(<FieldRow label="Timestamp" value="2026-01-18T12:00:00Z" />);

      const label = screen.getByText('Timestamp:');
      expect(label).toHaveClass('w-32');
    });
  });

  describe('edge cases', () => {
    it('should handle empty string value', () => {
      render(<FieldRow label="Empty Field" value="" />);

      expect(screen.getByText('Empty Field:')).toBeInTheDocument();
      // Copy button still exists for empty values
      expect(screen.getByRole('button', { name: /copy value/i })).toBeInTheDocument();
    });

    it('should handle very long label', () => {
      const longLabel = 'A'.repeat(50);
      render(<FieldRow label={longLabel} value="test" />);

      expect(screen.getByText(`${longLabel}:`)).toBeInTheDocument();
    });

    it('should handle very long value', () => {
      const longValue = 'B'.repeat(200);
      render(<FieldRow label="Test" value={longValue} />);

      const valueElement = screen.getByText(longValue);
      expect(valueElement).toHaveClass('truncate');
      expect(valueElement).toHaveAttribute('title', longValue);
    });

    it('should handle special characters in label', () => {
      render(<FieldRow label="Test: ñ 中文 🔥" value="value" />);

      expect(screen.getByText('Test: ñ 中文 🔥:')).toBeInTheDocument();
    });

    it('should handle special characters in value', () => {
      const specialValue = 'Value: ñ 中文 🔥';
      render(<FieldRow label="Special" value={specialValue} />);

      expect(screen.getByText(specialValue)).toBeInTheDocument();
    });

    it('should handle numeric values', () => {
      render(<FieldRow label="Port" value="443" />);

      expect(screen.getByText('443')).toBeInTheDocument();
    });

    it('should handle IP addresses', () => {
      render(<FieldRow label="IP" value="192.168.1.1" />);

      expect(screen.getByText('192.168.1.1')).toBeInTheDocument();
    });
  });

  describe('accessibility', () => {
    it('should render accessible copy button', () => {
      render(<FieldRow label="Test Field" value="test value" />);

      const button = screen.getByRole('button', { name: /copy value/i });
      expect(button).toBeInTheDocument();
    });

    it('should be keyboard navigable', () => {
      render(<FieldRow label="Test" value="value" />);

      const button = screen.getByRole('button', { name: /copy value/i });
      button.focus();
      expect(button).toHaveFocus();
    });
  });

  describe('dark mode support', () => {
    it('should have dark mode classes', () => {
      const { container } = render(<FieldRow label="Test" value="Value" />);

      const wrapper = container.firstChild;
      expect(wrapper).toHaveClass('dark:hover:bg-gray-800');
    });

    it('should apply dark mode text colors to label', () => {
      render(<FieldRow label="Label" value="Value" />);

      const label = screen.getByText('Label:');
      expect(label).toHaveClass('text-gray-600', 'dark:text-gray-400');
    });

    it('should apply dark mode text colors to value', () => {
      render(<FieldRow label="Label" value="Value" />);

      const value = screen.getByText('Value');
      expect(value).toHaveClass('text-gray-900', 'dark:text-gray-100');
    });
  });
});
