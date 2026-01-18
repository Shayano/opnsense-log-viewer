import { useEffect, useRef } from 'react';
import type { LogEntry } from '@/types/log-entry';
import { X } from 'lucide-react';
import { FieldRow } from './field-row';
import { CopyButton } from './copy-button';

interface EntryDetailViewProps {
  entry: LogEntry | null;
  isOpen: boolean;
  onClose: () => void;
  enrichmentData?: {
    interfaceNames?: Map<string, string>; // vtnet0 -> LAN
    ruleLabels?: Map<string, string>; // abc123 -> Block RFC1918
    aliases?: Map<string, string[]>; // IP -> alias names
  };
}

export function EntryDetailView({
  entry,
  isOpen,
  onClose,
  enrichmentData,
}: EntryDetailViewProps) {
  const detailRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  // Save previous focus when opening
  useEffect(() => {
    if (isOpen && !previousFocusRef.current) {
      previousFocusRef.current = document.activeElement as HTMLElement;
    }
  }, [isOpen]);

  // Focus trap implementation
  useEffect(() => {
    if (!isOpen || !detailRef.current) return;

    const handleTabKey = (e: KeyboardEvent) => {
      if (e.key !== 'Tab' || !detailRef.current) return;

      const focusableElements = detailRef.current.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      const firstElement = focusableElements[0] as HTMLElement;
      const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

      if (e.shiftKey) {
        // Shift+Tab: if focused on first element, wrap to last
        if (document.activeElement === firstElement) {
          e.preventDefault();
          lastElement?.focus();
        }
      } else {
        // Tab: if focused on last element, wrap to first
        if (document.activeElement === lastElement) {
          e.preventDefault();
          firstElement?.focus();
        }
      }
    };

    document.addEventListener('keydown', handleTabKey);
    return () => document.removeEventListener('keydown', handleTabKey);
  }, [isOpen]);

  // Restore focus when closing
  useEffect(() => {
    if (!isOpen && previousFocusRef.current) {
      previousFocusRef.current.focus();
      previousFocusRef.current = null;
    }
  }, [isOpen]);

  // Close on Esc key
  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEsc);
    return () => document.removeEventListener('keydown', handleEsc);
  }, [isOpen, onClose]);

  // Close on click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        detailRef.current &&
        !detailRef.current.contains(e.target as Node) &&
        isOpen
      ) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isOpen, onClose]);

  if (!isOpen || !entry) {
    return null;
  }

  // Get enriched values
  const interfaceDisplay = enrichmentData?.interfaceNames?.has(entry.interface)
    ? `${enrichmentData.interfaceNames.get(entry.interface)} (${entry.interface})`
    : entry.interface;

  const ruleLabelDisplay = enrichmentData?.ruleLabels?.has(entry.ruleLabel)
    ? `${enrichmentData.ruleLabels.get(entry.ruleLabel)} (${entry.ruleLabel})`
    : entry.ruleLabel;

  return (
    <div
      ref={detailRef}
      className={`
        fixed bottom-0 left-0 right-0 bg-white dark:bg-gray-900
        border-t border-gray-200 dark:border-gray-700
        transition-all duration-300 ease-in-out z-50
        ${isOpen ? 'h-[40vh]' : 'h-0'}
      `}
      role="complementary"
      aria-label="Log entry details"
    >
      <div className="flex flex-col h-full">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200 dark:border-gray-700">
          <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
            Entry Details
          </h3>
          <button
            onClick={onClose}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
            aria-label="Close detail view"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content - Split Layout */}
        <div className="flex-1 overflow-auto grid grid-cols-2 gap-4 p-4">
          {/* Left: Raw Log Line */}
          <div className="flex flex-col">
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase">
                Raw Log Line
              </h4>
              <CopyButton
                value={entry.rawLine || ''}
                label="Copy Full Entry"
              />
            </div>
            <textarea
              readOnly
              value={entry.rawLine || 'Raw log line not available'}
              className="
                flex-1 p-3 font-mono text-xs
                bg-gray-50 dark:bg-gray-800
                text-gray-900 dark:text-gray-100
                border border-gray-200 dark:border-gray-700
                rounded resize-none
              "
              aria-label="Raw log entry"
            />
          </div>

          {/* Right: Parsed Fields */}
          <div className="flex flex-col">
            <h4 className="text-xs font-semibold text-gray-600 dark:text-gray-400 uppercase mb-2">
              Parsed Fields
            </h4>
            <div className="flex-1 space-y-2 overflow-auto">
              <FieldRow label="Timestamp" value={entry.timestamp} />
              <FieldRow label="Interface" value={interfaceDisplay} />
              <FieldRow label="Source IP" value={entry.sourceIp} />
              <FieldRow
                label="Source Port"
                value={entry.sourcePort.toString()}
              />
              <FieldRow label="Destination IP" value={entry.destinationIp} />
              <FieldRow
                label="Destination Port"
                value={entry.destinationPort.toString()}
              />
              <FieldRow
                label="Protocol"
                value={entry.protocol.toUpperCase()}
              />
              <FieldRow label="Action" value={entry.action.toUpperCase()} />
              <FieldRow label="Rule Label" value={ruleLabelDisplay} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
