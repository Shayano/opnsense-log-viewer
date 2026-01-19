import { useEnrichmentStore } from '@/stores/enrichment-store';
import { Wifi, WifiOff, WifiLow } from 'lucide-react';

export function ConnectionIndicator() {
  const connectionStatus = useEnrichmentStore((state) => state.connectionStatus);
  const lastError = useEnrichmentStore((state) => state.lastError);

  if (!connectionStatus) {
    return null;
  }

  const getStatusConfig = () => {
    switch (connectionStatus) {
      case 'connected':
        return {
          icon: Wifi,
          label: 'Connected',
          color: 'text-green-600 dark:text-green-400',
          dotColor: 'bg-green-600 dark:bg-green-400',
        };
      case 'degraded':
        return {
          icon: WifiLow,
          label: 'Degraded',
          color: 'text-yellow-600 dark:text-yellow-400',
          dotColor: 'bg-yellow-600 dark:bg-yellow-400',
        };
      case 'disconnected':
        return {
          icon: WifiOff,
          label: 'Offline',
          color: 'text-red-600 dark:text-red-400',
          dotColor: 'bg-red-600 dark:bg-red-400',
        };
      default:
        // Fallback for unexpected status
        return {
          icon: WifiOff,
          label: 'Unknown',
          color: 'text-gray-600 dark:text-gray-400',
          dotColor: 'bg-gray-600 dark:bg-gray-400',
        };
    }
  };

  const config = getStatusConfig();
  const Icon = config.icon;

  return (
    <div
      className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-gray-100 dark:bg-gray-800 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
      title={lastError || `API ${config.label}`}
    >
      <div className="relative">
        <Icon className={`h-4 w-4 ${config.color}`} />
        <div className={`absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full ${config.dotColor}`} />
      </div>
      <span className={`text-sm font-medium ${config.color}`}>
        {config.label}
      </span>
    </div>
  );
}
