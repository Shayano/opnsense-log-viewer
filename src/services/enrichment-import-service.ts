import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';
import { useEnrichmentStore } from '@/stores/enrichment-store';

interface ImportValidation {
  isValid: boolean;
  errorMessage?: string;
  missingFields?: string[];
  ageDays?: number;
  isStale: boolean;
  metadata?: {
    exportTimestamp: string;
    deviceId: string;
    opnsenseVersion?: string;
    configHash: string;
    appVersion: string;
    dataSource: string;
    cacheStatus: {
      interfacesCount: number;
      rulesCount: number;
      aliasesCount: number;
    };
  };
}

interface ImportResult {
  interfacesImported: number;
  rulesImported: number;
  aliasesImported: number;
  exportTimestamp: string;
  deviceId: string;
  ageDays: number;
}

/**
 * Import enrichment data workflow
 *
 * @returns true if import succeeded, false if cancelled/failed
 */
export async function importEnrichmentData(): Promise<boolean> {
  console.log('[Import] Starting enrichment data import workflow');

  try {
    // Step 1: Open file picker
    console.log('[Import] Opening file picker dialog');
    const filePath = await invoke<string | null>('open_enrichment_file_picker');

    if (!filePath) {
      console.log('[Import] User cancelled file picker');
      return false; // User cancelled file picker
    }

    console.log('[Import] Selected file:', filePath);

    // Step 2: Validate enrichment file
    console.log('[Import] Validating enrichment file');
    const validation = await invoke<ImportValidation>('validate_enrichment_import', {
      filePath,
    });

    if (!validation.isValid) {
      // Show validation error
      showValidationError(validation);
      return false;
    }

    console.log('[Import] Validation passed:', {
      ageDays: validation.ageDays,
      isStale: validation.isStale,
    });

    // Step 3: Check for staleness (>7 days)
    if (validation.isStale && validation.ageDays && validation.metadata) {
      const proceed = await showStaleEnrichmentWarning({
        ageDays: validation.ageDays,
        exportTimestamp: validation.metadata.exportTimestamp,
        deviceId: validation.metadata.deviceId,
      });

      if (!proceed) {
        return false; // User cancelled due to staleness
      }
    }

    // Step 4: Import enrichment data
    const result = await invoke<ImportResult>('import_enrichment_data', {
      filePath,
    });

    // Step 5: Update enrichment store with backup metadata
    const store = useEnrichmentStore.getState();
    store.setBackupEnrichment({
      importedAt: new Date().toISOString(),
      ageDays: result.ageDays,
      sourceFile: filePath,
      exportTimestamp: result.exportTimestamp,
      deviceId: result.deviceId,
    });

    // Step 5b: Reload all enrichment data from backend cache to frontend store
    console.log('[Import] Reloading enrichment data into frontend store');
    try {
      // Reload interface mappings
      const interfaceMappings = await invoke<any>('get_interface_mappings_cmd');
      if (interfaceMappings) {
        store.setInterfaceMappings(interfaceMappings);
        console.log('[Import] Interface mappings reloaded:', interfaceMappings.mappings);
      }

      // Reload rule labels
      const ruleLabels = await invoke<Record<string, string>>('get_rule_labels');
      if (ruleLabels) {
        store.setRuleLabels(new Map(Object.entries(ruleLabels)));
        console.log('[Import] Rule labels reloaded:', Object.keys(ruleLabels).length, 'rules');
      }

      // Reload aliases
      const aliases = await invoke<Record<string, any>>('get_aliases');
      if (aliases) {
        // Convert aliases to Map format expected by store
        const aliasesMap = new Map(Object.entries(aliases));
        store.setAliases(aliasesMap);
        console.log('[Import] Aliases reloaded:', Object.keys(aliases).length, 'aliases');
      }
    } catch (error) {
      console.warn('[Import] Failed to reload some enrichment data:', error);
      // Don't fail the import if reload fails - data is already in backend cache
    }

    // Step 6: Show success notification
    toast.success(
      `Backup enrichment loaded successfully\n${result.interfacesImported} interfaces, ${result.rulesImported} rules, ${result.aliasesImported} aliases`,
      { duration: 4000 }
    );

    console.log('[Import] Successfully imported enrichment data:', {
      interfaces: result.interfacesImported,
      rules: result.rulesImported,
      aliases: result.aliasesImported,
      ageDays: result.ageDays,
      deviceId: result.deviceId,
    });

    return true;

  } catch (error) {
    console.error('[Import] Import failed:', error);

    // Provide more specific error messages
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (errorMessage.includes('Validation failed') || errorMessage.includes('Invalid JSON')) {
      toast.error(`Import validation failed: ${errorMessage}`);
    } else if (errorMessage.includes('Failed to read file')) {
      toast.error(`Could not read file: ${errorMessage}`);
    } else {
      toast.error(`Import failed: ${errorMessage}`);
    }

    return false;
  }
}

/**
 * Show validation error dialog
 */
function showValidationError(validation: ImportValidation): void {
  console.error('[Import] Validation error:', validation);

  let message = validation.errorMessage || 'Validation failed';

  if (validation.missingFields && validation.missingFields.length > 0) {
    message += `\n\nMissing fields: ${validation.missingFields.join(', ')}`;
  }

  message += '\n\nTry exporting a new enrichment file.';

  // Use browser alert for now (will be replaced with modal dialog in Story 4.3)
  alert(message);
}

/**
 * Show staleness warning dialog
 */
async function showStaleEnrichmentWarning(data: {
  ageDays: number;
  exportTimestamp: string;
  deviceId: string;
}): Promise<boolean> {
  console.warn('[Import] Stale enrichment warning:', data);

  const exportDate = new Date(data.exportTimestamp).toLocaleDateString();

  const message =
    `⚠️ Enrichment data from ${exportDate} (${data.ageDays} days old)\n\n` +
    `Interface mappings and rule labels may be outdated.\n` +
    `Verify accuracy for critical investigations.\n\n` +
    `Continue?`;

  // Use browser confirm for now (will be replaced with modal dialog in Story 4.3)
  const result = confirm(message);
  console.log('[Import] User staleness warning response:', result ? 'Continue' : 'Cancel');
  return result;
}
