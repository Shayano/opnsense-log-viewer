import { WifiOff, Loader2, CheckCircle, XCircle } from 'lucide-react';
import type { ConnectionStatus, ConnectionTestResult } from '../../types/api';

interface ConnectionStatusBadgeProps {
  status: ConnectionStatus;
  result: ConnectionTestResult | null;
}

/**
 * Connection status indicator badge
 * Story 3.1: API Connection Setup with Credential Storage
 */
export function ConnectionStatusBadge({ status, result }: ConnectionStatusBadgeProps) {
  const statusConfig = {
    disconnected: {
      icon: WifiOff,
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-100 dark:bg-red-900/30',
      label: 'Disconnected',
      animated: false,
    },
    connecting: {
      icon: Loader2,
      color: 'text-yellow-600 dark:text-yellow-400',
      bgColor: 'bg-yellow-100 dark:bg-yellow-900/30',
      label: 'Connecting...',
      animated: true,
    },
    connected: {
      icon: CheckCircle,
      color: 'text-green-600 dark:text-green-400',
      bgColor: 'bg-green-100 dark:bg-green-900/30',
      label: result?.opnsenseVersion
        ? `Connected to OPNsense v${result.opnsenseVersion}`
        : 'Connected',
      animated: false,
    },
    error: {
      icon: XCircle,
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-100 dark:bg-red-900/30',
      label: 'Connection Failed',
      animated: false,
    },
  };

  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <div className={`inline-flex items-center gap-2 px-3 py-2 rounded-lg ${config.bgColor}`}>
      <Icon className={`w-5 h-5 ${config.color} ${config.animated ? 'animate-spin' : ''}`} />
      <span className={`text-sm font-medium ${config.color}`}>{config.label}</span>
    </div>
  );
}
