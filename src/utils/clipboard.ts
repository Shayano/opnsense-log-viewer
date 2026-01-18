/**
 * Copy text to clipboard with fallback for older browsers
 * @param text Text to copy
 * @returns Promise<boolean> Success status
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    // Modern Clipboard API (preferred)
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return true;
    }

    // Fallback for older browsers
    return fallbackCopyToClipboard(text);
  } catch (error) {
    console.error('Clipboard copy failed:', error);
    return fallbackCopyToClipboard(text);
  }
}

/**
 * Fallback clipboard copy using deprecated document.execCommand
 */
function fallbackCopyToClipboard(text: string): boolean {
  try {
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    const success = document.execCommand('copy');
    document.body.removeChild(textarea);
    return success;
  } catch (error) {
    console.error('Fallback clipboard copy failed:', error);
    return false;
  }
}
