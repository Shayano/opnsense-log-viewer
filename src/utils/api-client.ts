import { invoke } from '@tauri-apps/api/core';
import type { ApiCredentials, ConnectionTestResult } from '../types/api';

/**
 * Save API credentials to secure storage (OS keychain or encrypted file)
 * Story 3.1: API Connection Setup with Credential Storage
 */
export async function saveApiCredentials(
  endpointUrl: string,
  apiKey: string,
  apiSecret: string,
  acceptInvalidCerts: boolean = false
): Promise<void> {
  await invoke('save_api_credentials', {
    endpointUrl,
    apiKey,
    apiSecret,
    acceptInvalidCerts,
  });
}

/**
 * Load API credentials from secure storage
 */
export async function loadApiCredentials(): Promise<ApiCredentials | null> {
  return await invoke<ApiCredentials | null>('load_api_credentials');
}

/**
 * Test connection to OPNsense API
 */
export async function testApiConnection(
  endpointUrl: string,
  apiKey: string,
  apiSecret: string,
  acceptInvalidCerts: boolean = false
): Promise<ConnectionTestResult> {
  return await invoke<ConnectionTestResult>('test_api_connection', {
    endpointUrl,
    apiKey,
    apiSecret,
    acceptInvalidCerts,
  });
}
