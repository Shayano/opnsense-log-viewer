import { invoke } from '@tauri-apps/api/core';
import type { AliasMapping } from '@/types/api';
import { LogEntry } from '@/types/log-entry';
import { extractUniqueRuleHashes } from '@/utils/extract-rule-hashes';
import { extractUniqueIPs } from '@/utils/extract-ips';
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

// ============================================================================
// Alias Resolution (Story 3.4)
// ============================================================================

/**
 * Enrich IP aliases for loaded log entries
 *
 * Extracts unique IPs, fetches aliases from OPNsense API,
 * and updates the enrichment store
 *
 * @param entries - Log entries to enrich
 */
export async function enrichAliases(entries: LogEntry[]): Promise<void> {
  // Extract unique IPs (source + destination)
  const ips = extractUniqueIPs(entries);

  if (ips.size === 0) {
    console.log('No IPs found in entries');
    return;
  }

  const ipArray = Array.from(ips);
  console.log(`Enriching ${ipArray.length} unique IP aliases`);

  try {
    // Show progress toast
    const toastId = toast.loading(`Enriching aliases... ${ipArray.length} IPs`);

    // Fetch aliases from backend
    const aliases = await invoke<Record<string, AliasMapping[]>>('fetch_aliases', {
      ips: ipArray,
    });

    // Update store
    useEnrichmentStore.getState().setAliases(aliases);

    const aliasedCount = Object.keys(aliases).length;
    const notAliasedCount = ipArray.length - aliasedCount;

    // Success toast
    toast.success(
      `IP aliases enriched (${aliasedCount} of ${ipArray.length} aliased)`,
      { id: toastId }
    );

    if (notAliasedCount > 0) {
      console.log(`${notAliasedCount} IPs not aliased`);
    }
  } catch (error) {
    console.error('Failed to enrich aliases:', error);

    // Provide specific error guidance
    const errorMessage = (error as Error).toString();
    if (errorMessage.includes('No API credentials')) {
      toast.error('Alias enrichment failed: No API credentials configured. Configure OPNsense connection in Settings.');
    } else if (errorMessage.includes('Authentication failed') || errorMessage.includes('401')) {
      toast.error('Alias enrichment failed: Invalid API credentials. Check your API key and secret in Settings.');
    } else if (errorMessage.includes('Network') || errorMessage.includes('timeout')) {
      toast.error('Alias enrichment failed: Cannot reach OPNsense API. Check network connection.');
    } else {
      toast.error('Alias enrichment failed. Using cached data.');
    }
  }
}

/**
 * Load cached aliases on app startup
 */
export async function loadCachedAliases(): Promise<void> {
  try {
    const aliases = await invoke<Record<string, AliasMapping[]>>('get_aliases');

    if (Object.keys(aliases).length > 0) {
      useEnrichmentStore.getState().setAliases(aliases);
      console.log(`Loaded ${Object.keys(aliases).length} cached IP aliases`);
    }
  } catch (error) {
    console.error('Failed to load cached aliases:', error);
  }
}
