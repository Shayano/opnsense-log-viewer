import { useState, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { Wifi, Loader2, Eye, EyeOff, Download } from 'lucide-react';
import toast from 'react-hot-toast';
import { ConnectionStatusBadge } from './connection-status-badge';
import {
  saveApiCredentials,
  loadApiCredentials,
  testApiConnection,
} from '../../utils/api-client';
import type { ConnectionStatus, ConnectionTestResult } from '../../types/api';
import { exportEnrichmentData } from '../../services/enrichment-export';
import { useEnrichmentStore } from '../../stores/enrichment-store';

interface ApiCredentialsForm {
  endpointUrl: string;
  apiKey: string;
  apiSecret: string;
  acceptInvalidCerts: boolean;
}

/**
 * API Configuration Settings Component
 * Story 3.1: API Connection Setup with Credential Storage
 */
export function ApiConfigSettings() {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected');
  const [showApiKey, setShowApiKey] = useState(false);
  const [showApiSecret, setShowApiSecret] = useState(false);
  const [testResult, setTestResult] = useState<ConnectionTestResult | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  // Get enrichment data from store
  const interfaceMappings = useEnrichmentStore((state) => state.interfaceMappings);
  const ruleLabels = useEnrichmentStore((state) => state.ruleLabels);
  const aliases = useEnrichmentStore((state) => state.aliases);

  const {
    register,
    handleSubmit,
    setValue,
    formState: { errors, isValid },
  } = useForm<ApiCredentialsForm>({
    mode: 'onChange',
  });

  // Load credentials on mount
  useEffect(() => {
    loadCredentialsFromStorage();
  }, []);

  const loadCredentialsFromStorage = async () => {
    try {
      const credentials = await loadApiCredentials();
      if (credentials) {
        setValue('endpointUrl', credentials.endpointUrl);
        setValue('apiKey', credentials.apiKey);
        setValue('apiSecret', credentials.apiSecret);
        setValue('acceptInvalidCerts', credentials.acceptInvalidCerts ?? false);

        // Auto-connect in background
        setConnectionStatus('connecting');
        testConnectionSilent({ ...credentials, acceptInvalidCerts: credentials.acceptInvalidCerts ?? false });
      }
    } catch (error) {
      console.error('Failed to load credentials:', error);
    }
  };

  const testConnectionSilent = async (credentials: ApiCredentialsForm) => {
    try {
      const result = await testApiConnection(
        credentials.endpointUrl,
        credentials.apiKey,
        credentials.apiSecret,
        credentials.acceptInvalidCerts
      );

      if (result.success) {
        setConnectionStatus('connected');
        setTestResult(result);
      } else {
        setConnectionStatus('error');
      }
    } catch (error) {
      setConnectionStatus('error');
    }
  };

  const onTestConnection = async (data: ApiCredentialsForm) => {
    setIsTesting(true);
    setConnectionStatus('connecting');

    try {
      const result = await testApiConnection(
        data.endpointUrl,
        data.apiKey,
        data.apiSecret,
        data.acceptInvalidCerts
      );

      if (result.success) {
        setConnectionStatus('connected');
        setTestResult(result);
        toast.success(
          `Connected to OPNsense${result.opnsenseVersion ? ` v${result.opnsenseVersion}` : ''}. Found ${result.interfaceCount} interfaces.`
        );
      } else {
        setConnectionStatus('error');
        toast.error(result.errorMessage || 'Connection test failed');
      }
    } catch (error) {
      setConnectionStatus('error');
      toast.error(String(error));
    } finally {
      setIsTesting(false);
    }
  };

  const onSave = async (data: ApiCredentialsForm) => {
    setIsSaving(true);

    try {
      await saveApiCredentials(
        data.endpointUrl,
        data.apiKey,
        data.apiSecret,
        data.acceptInvalidCerts
      );
      toast.success('Credentials saved securely');
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsSaving(false);
    }
  };

  const handleExport = async () => {
    setIsExporting(true);
    try {
      await exportEnrichmentData();
    } finally {
      setIsExporting(false);
    }
  };

  // Calculate cache counts for display
  const interfaceCount = interfaceMappings.size;
  const ruleCount = ruleLabels.size;
  const aliasCount = aliases.size;
  const cacheSource = connectionStatus === 'connected' ? 'Live API' : 'Cached';

  return (
    <div className="max-w-2xl mx-auto p-6 bg-white dark:bg-gray-900 rounded-lg shadow">
      <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-6">
        OPNsense API Configuration
      </h2>

      {/* Connection Status Indicator */}
      <div className="mb-6 flex items-center gap-3">
        <ConnectionStatusBadge status={connectionStatus} result={testResult} />
      </div>

      <form className="space-y-4">
        {/* Endpoint URL */}
        <div>
          <label
            htmlFor="endpointUrl"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Endpoint URL
          </label>
          <input
            id="endpointUrl"
            type="text"
            placeholder="https://192.168.1.1"
            {...register('endpointUrl', {
              required: 'Endpoint URL is required',
              pattern: {
                value: /^https:\/\/.+/,
                message: 'Must start with https://',
              },
            })}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700
              bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
              rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          {errors.endpointUrl && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">
              {errors.endpointUrl.message}
            </p>
          )}
        </div>

        {/* API Key */}
        <div>
          <label
            htmlFor="apiKey"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            API Key
          </label>
          <div className="relative">
            <input
              id="apiKey"
              type={showApiKey ? 'text' : 'password'}
              {...register('apiKey', { required: 'API Key is required' })}
              className="w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-700
                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
                rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            <button
              type="button"
              onClick={() => setShowApiKey(!showApiKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1
                text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
            >
              {showApiKey ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
            </button>
          </div>
          {errors.apiKey && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">{errors.apiKey.message}</p>
          )}
        </div>

        {/* API Secret */}
        <div>
          <label
            htmlFor="apiSecret"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            API Secret
          </label>
          <div className="relative">
            <input
              id="apiSecret"
              type={showApiSecret ? 'text' : 'password'}
              {...register('apiSecret', { required: 'API Secret is required' })}
              className="w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-700
                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
                rounded focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            <button
              type="button"
              onClick={() => setShowApiSecret(!showApiSecret)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1
                text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
            >
              {showApiSecret ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
            </button>
          </div>
          {errors.apiSecret && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">
              {errors.apiSecret.message}
            </p>
          )}
        </div>

        {/* Accept Invalid Certificates Checkbox */}
        <div className="flex items-start gap-2 pt-2">
          <input
            id="acceptInvalidCerts"
            type="checkbox"
            {...register('acceptInvalidCerts')}
            className="mt-1 w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded
              focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800
              focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
          />
          <label
            htmlFor="acceptInvalidCerts"
            className="text-sm text-gray-700 dark:text-gray-300 cursor-pointer"
          >
            <span className="font-medium">Accept Invalid Certificates</span>
            <span className="block text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              Allow self-signed or invalid TLS certificates. Use only for trusted local networks.
            </span>
          </label>
        </div>

        {/* Action Buttons */}
        <div className="flex gap-3 pt-4">
          <button
            type="button"
            onClick={handleSubmit(onTestConnection)}
            disabled={!isValid || isTesting}
            className="px-4 py-2 bg-blue-600 text-white rounded
              hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed
              flex items-center gap-2 transition-colors"
          >
            {isTesting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Testing...
              </>
            ) : (
              <>
                <Wifi className="w-4 h-4" />
                Test Connection
              </>
            )}
          </button>

          <button
            type="button"
            onClick={handleSubmit(onSave)}
            disabled={!isValid || isSaving}
            className="px-4 py-2 bg-green-600 text-white rounded
              hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed
              transition-colors"
          >
            {isSaving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </form>

      {/* Export Section (Story 4.1) */}
      <div className="border-t border-gray-200 dark:border-gray-700 mt-8 pt-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">
          Enrichment Data Export
        </h3>

        <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
          Export interface mappings, rule labels, and aliases to a JSON file for offline use.
        </p>

        {/* Cache Status Display */}
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 mb-4 text-sm">
          <div className="flex items-center justify-between mb-2">
            <span className="text-gray-700 dark:text-gray-300 font-medium">Current cache:</span>
            <span className="text-gray-500 dark:text-gray-400">{cacheSource}</span>
          </div>
          <div className="space-y-1 text-gray-600 dark:text-gray-400">
            <div className="flex justify-between">
              <span>Interfaces:</span>
              <span className="font-mono font-medium">{interfaceCount}</span>
            </div>
            <div className="flex justify-between">
              <span>Rule labels:</span>
              <span className="font-mono font-medium">{ruleCount}</span>
            </div>
            <div className="flex justify-between">
              <span>Aliases:</span>
              <span className="font-mono font-medium">{aliasCount}</span>
            </div>
          </div>
        </div>

        <button
          onClick={handleExport}
          disabled={isExporting}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200
            bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700
            rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          title="Export enrichment data for offline use"
        >
          {isExporting ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Exporting...
            </>
          ) : (
            <>
              <Download className="h-4 w-4" />
              Export Enrichment Data
            </>
          )}
        </button>
      </div>
    </div>
  );
}
