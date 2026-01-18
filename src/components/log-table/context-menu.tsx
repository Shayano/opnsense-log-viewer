import React, { useEffect, useRef, useState } from 'react';
import { Copy, Filter } from 'lucide-react';
import type { LogEntry } from '@/types/log-entry';
import toast from 'react-hot-toast';

interface ContextMenuProps {
  position: { x: number; y: number };
  entry: LogEntry | null;
  cellValue?: string;
  onClose: () => void;
  onFilterByValue?: (value: string) => void;
}

export function ContextMenu({
  position,
  entry,
  cellValue,
  onClose,
  onFilterByValue,
}: ContextMenuProps): React.JSX.Element | null {
  const menuRef = useRef<HTMLDivElement>(null);
  const [adjustedPosition, setAdjustedPosition] = useState(position);

  // Adjust position to keep menu within viewport
  useEffect(() => {
    if (menuRef.current) {
      const menu = menuRef.current;
      const menuRect = menu.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let newX = position.x;
      let newY = position.y;

      // Check if menu goes off right edge
      if (position.x + menuRect.width > viewportWidth) {
        newX = viewportWidth - menuRect.width - 8;
      }

      // Check if menu goes off bottom edge
      if (position.y + menuRect.height > viewportHeight) {
        newY = viewportHeight - menuRect.height - 8;
      }

      // Ensure menu doesn't go off left or top edge
      newX = Math.max(8, newX);
      newY = Math.max(8, newY);

      if (newX !== position.x || newY !== position.y) {
        setAdjustedPosition({ x: newX, y: newY });
      }
    }
  }, [position]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent): void => {
      const target = event.target;
      if (menuRef.current && target instanceof HTMLElement && !menuRef.current.contains(target)) {
        onClose();
      }
    };

    const handleEscape = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [onClose]);

  if (!entry) return null;

  const handleCopyCell = async (): Promise<void> => {
    if (cellValue) {
      try {
        await window.navigator.clipboard.writeText(cellValue);
        toast.success('Cell value copied to clipboard');
      } catch {
        toast.error('Failed to copy cell value');
      }
      onClose();
    }
  };

  const handleCopyRow = async (): Promise<void> => {
    const rowText = `${entry.timestamp} | ${entry.interface} | ${entry.sourceIp}:${entry.sourcePort} -> ${entry.destinationIp}:${entry.destinationPort} | ${entry.protocol} | ${entry.action} | ${entry.ruleLabel}`;
    try {
      await window.navigator.clipboard.writeText(rowText);
      toast.success('Row copied to clipboard');
    } catch {
      toast.error('Failed to copy row');
    }
    onClose();
  };

  const handleFilterByValue = (): void => {
    if (cellValue && onFilterByValue) {
      onFilterByValue(cellValue);
      toast.success(`Filter by "${cellValue}" will be applied`);
    }
    onClose();
  };

  const menuItemClass =
    'flex items-center gap-2 px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer transition-colors';

  return (
    <div
      ref={menuRef}
      className="fixed z-50 min-w-[200px] bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg"
      style={{
        top: `${adjustedPosition.y}px`,
        left: `${adjustedPosition.x}px`,
      }}
      role="menu"
      aria-label="Context menu"
    >
      <div
        className={menuItemClass}
        onClick={handleCopyCell}
        role="menuitem"
        tabIndex={0}
        aria-label="Copy cell value"
      >
        <Copy className="w-4 h-4" aria-hidden="true" />
        <span>Copy Cell Value</span>
      </div>

      <div
        className={menuItemClass}
        onClick={handleCopyRow}
        role="menuitem"
        tabIndex={0}
        aria-label="Copy entire row"
      >
        <Copy className="w-4 h-4" aria-hidden="true" />
        <span>Copy Row</span>
      </div>

      <div className="border-t border-gray-200 dark:border-gray-700"></div>

      <div
        className={menuItemClass}
        onClick={handleFilterByValue}
        role="menuitem"
        tabIndex={0}
        aria-label="Filter by this value"
      >
        <Filter className="w-4 h-4" aria-hidden="true" />
        <span>Filter by This Value</span>
      </div>
    </div>
  );
}
