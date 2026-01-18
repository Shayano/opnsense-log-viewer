import { Copy, Check } from 'lucide-react';
import { useState } from 'react';
import { copyToClipboard } from '@/utils/clipboard';
import toast from 'react-hot-toast';

interface CopyButtonProps {
  value: string;
  label?: string;
  compact?: boolean;
}

export function CopyButton({ value, label, compact = false }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    const success = await copyToClipboard(value);

    if (success) {
      setCopied(true);
      toast.success('Copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } else {
      toast.error('Failed to copy to clipboard');
    }
  };

  if (compact) {
    return (
      <button
        onClick={handleCopy}
        className="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-all active:scale-95 active:bg-gray-300 dark:active:bg-gray-600"
        aria-label={`Copy ${label || 'value'}`}
      >
        {copied ? (
          <Check className="w-3 h-3 text-green-600 dark:text-green-400" />
        ) : (
          <Copy className="w-3 h-3 text-gray-600 dark:text-gray-400" />
        )}
      </button>
    );
  }

  return (
    <button
      onClick={handleCopy}
      className="
        flex items-center gap-1 px-2 py-1 text-xs
        bg-gray-100 dark:bg-gray-800
        hover:bg-gray-200 dark:hover:bg-gray-700
        text-gray-700 dark:text-gray-300
        rounded transition-all
        active:scale-95 active:bg-gray-300 dark:active:bg-gray-600
      "
      aria-label={`Copy ${label || 'value'}`}
    >
      {copied ? (
        <>
          <Check className="w-3 h-3 text-green-600 dark:text-green-400" />
          <span>Copied</span>
        </>
      ) : (
        <>
          <Copy className="w-3 h-3" />
          <span>{label || 'Copy'}</span>
        </>
      )}
    </button>
  );
}
