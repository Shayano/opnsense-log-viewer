import { invoke } from '@tauri-apps/api/core';
import { LogEntry } from '@/types/log-entry';
import { extractUniqueRuleHashes } from '@/utils/extract-rule-hashes';
import { useEnrichmentStore } from '@/stores/enrichment-store';
import toast from 'react-hot-toast';

/**
 * Enrich rule labels for loaded log entries
 *
 * Extracts unique rule hashes, fetches labels from OPNsense API,
 * and updates the enrichment store
 *
 * @param entries - Log entries to enrich
 */
export async function enrichRuleLabels(entries: LogEntry[]): Promise<void> {
  // Extract unique hashes
  const hashes = extractUniqueRuleHashes(entries);

  if (hashes.size === 0) {
    console.log('No rule hashes found in entries');
    return;
  }

  const hashArray = Array.from(hashes);
  console.log(`Enriching ${hashArray.length} unique rule labels`);

  try {
    // Show progress toast
    const toastId = toast.loading(`Enriching rules... 0 of ${hashArray.length}`);

    // Fetch labels from backend
    const labels = await invoke<Record<string, string>>('fetch_rule_labels', {
      hashes: hashArray,
    });

    // Update store
    useEnrichmentStore.getState().setRuleLabels(labels);

    const foundCount = Object.keys(labels).length;
    const notFoundCount = hashArray.length - foundCount;

    // Success toast
    toast.success(
      `Rule labels enriched (${foundCount} of ${hashArray.length} found)`,
      { id: toastId }
    );

    if (notFoundCount > 0) {
      console.warn(`${notFoundCount} rule labels not found in OPNsense`);
    }
  } catch (error) {
    console.error('Failed to enrich rule labels:', error);

    // Provide specific error guidance based on error type
    const errorMessage = (error as Error).toString();
    if (errorMessage.includes('No API credentials')) {
      toast.error('Rule enrichment failed: No API credentials configured. Configure OPNsense connection in Settings.');
    } else if (errorMessage.includes('Authentication failed') || errorMessage.includes('401')) {
      toast.error('Rule enrichment failed: Invalid API credentials. Check your API key and secret in Settings.');
    } else if (errorMessage.includes('Network') || errorMessage.includes('timeout')) {
      toast.error('Rule enrichment failed: Cannot reach OPNsense API. Check network connection and firewall settings.');
    } else {
      toast.error(`Rule enrichment failed: ${errorMessage}. Using cached labels.`);
    }
  }
}

/**
 * Load cached rule labels on app startup
 */
export async function loadCachedRuleLabels(): Promise<void> {
  try {
    const labels = await invoke<Record<string, string>>('get_rule_labels');

    if (Object.keys(labels).length > 0) {
      useEnrichmentStore.getState().setRuleLabels(labels);
      console.log(`Loaded ${Object.keys(labels).length} cached rule labels`);
    }
  } catch (error) {
    console.error('Failed to load cached rule labels:', error);
  }
}
