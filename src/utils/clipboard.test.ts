import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { copyToClipboard } from './clipboard';

describe('copyToClipboard', () => {
  let mockWriteText: ReturnType<typeof vi.fn>;
  let mockExecCommand: ReturnType<typeof vi.fn>;
  let originalClipboard: typeof navigator.clipboard;
  let originalIsSecureContext: boolean;

  beforeEach(() => {
    // Save original values
    originalClipboard = navigator.clipboard;
    originalIsSecureContext = window.isSecureContext;

    // Mock clipboard API
    mockWriteText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: {
        writeText: mockWriteText,
      },
      writable: true,
      configurable: true,
    });

    // Mock secure context
    Object.defineProperty(window, 'isSecureContext', {
      value: true,
      writable: true,
      configurable: true,
    });

    // Mock execCommand
    mockExecCommand = vi.fn().mockReturnValue(true);
    document.execCommand = mockExecCommand;
  });

  afterEach(() => {
    // Restore original values
    Object.defineProperty(navigator, 'clipboard', {
      value: originalClipboard,
      writable: true,
      configurable: true,
    });
    Object.defineProperty(window, 'isSecureContext', {
      value: originalIsSecureContext,
      writable: true,
      configurable: true,
    });
    vi.restoreAllMocks();
  });

  it('should copy text using modern Clipboard API when available', async () => {
    const testText = 'Test log entry';
    const result = await copyToClipboard(testText);

    expect(result).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith(testText);
    expect(mockExecCommand).not.toHaveBeenCalled();
  });

  it('should use fallback when Clipboard API is not available', async () => {
    // Remove clipboard API
    Object.defineProperty(navigator, 'clipboard', {
      value: undefined,
      writable: true,
      configurable: true,
    });

    const testText = 'Test log entry';
    const result = await copyToClipboard(testText);

    expect(result).toBe(true);
    expect(mockWriteText).not.toHaveBeenCalled();
    expect(mockExecCommand).toHaveBeenCalledWith('copy');
  });

  it('should use fallback when not in secure context', async () => {
    Object.defineProperty(window, 'isSecureContext', {
      value: false,
      writable: true,
      configurable: true,
    });

    const testText = 'Test log entry';
    const result = await copyToClipboard(testText);

    expect(result).toBe(true);
    expect(mockWriteText).not.toHaveBeenCalled();
    expect(mockExecCommand).toHaveBeenCalledWith('copy');
  });

  it('should return false when Clipboard API fails and fallback fails', async () => {
    mockWriteText.mockRejectedValue(new Error('Clipboard API failed'));
    mockExecCommand.mockReturnValue(false);

    const testText = 'Test log entry';
    const result = await copyToClipboard(testText);

    expect(result).toBe(false);
  });

  it('should handle empty string', async () => {
    const result = await copyToClipboard('');

    expect(result).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith('');
  });

  it('should handle special characters', async () => {
    const testText = 'Special chars: ñ 中文 🔥 \n\t\r';
    const result = await copyToClipboard(testText);

    expect(result).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith(testText);
  });

  it('should handle very long text', async () => {
    const testText = 'A'.repeat(100000);
    const result = await copyToClipboard(testText);

    expect(result).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith(testText);
  });
});
