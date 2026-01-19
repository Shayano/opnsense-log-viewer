import * as Tooltip from '@radix-ui/react-tooltip';
import { AlertTriangle } from 'lucide-react';
import { useEnrichmentStore } from '@/stores/enrichment-store';

export function MinimizedStalenessIcon() {
  const stalenessIndicatorDismissed = useEnrichmentStore((state) => state.stalenessIndicatorDismissed);
  const backupEnrichmentActive = useEnrichmentStore((state) => state.backupEnrichmentActive);
  const setStalenessIndicatorDismissed = useEnrichmentStore((state) => state.setStalenessIndicatorDismissed);

  // Only show if dismissed and backup enrichment is active
  if (!stalenessIndicatorDismissed || !backupEnrichmentActive) {
    return null;
  }

  const handleClick = () => {
    setStalenessIndicatorDismissed(false);
  };

  return (
    <Tooltip.Provider>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            onClick={handleClick}
            className="fixed top-16 right-4 z-40 p-2 rounded-md bg-amber-100 dark:bg-amber-900/30 text-amber-800 dark:text-amber-200 hover:bg-amber-200 dark:hover:bg-amber-900/40 transition-colors"
            aria-label="Show backup enrichment warning"
          >
            <AlertTriangle className="h-4 w-4" />
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            className="z-50 rounded-md bg-gray-900 dark:bg-gray-800 px-3 py-2 text-sm text-white shadow-lg"
            sideOffset={5}
          >
            Show backup enrichment warning
            <Tooltip.Arrow className="fill-gray-900 dark:fill-gray-800" />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}
