/**
 * API credentials for OPNsense connection
 * Story 3.1: API Connection Setup with Credential Storage
 */
export interface ApiCredentials {
  endpointUrl: string;
  apiKey: string;
  apiSecret: string;
  profileName?: string;
}

/**
 * Connection status for UI indicator
 */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

/**
 * Result from test connection
 */
export interface ConnectionTestResult {
  success: boolean;
  interfaceCount: number;
  opnsenseVersion?: string;
  errorMessage?: string;
}
