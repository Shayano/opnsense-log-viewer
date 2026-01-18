import { useEnrichmentStore } from '@/stores/enrichment-store';

interface InterfaceNameResult {
  logicalName: string | null;
  physicalName: string;
  displayName: string;
  tooltipText: string;
}

/**
 * Hook to resolve interface names (physical → logical)
 *
 * @param physicalName - Physical interface name (e.g., "vtnet0")
 * @returns InterfaceNameResult with logical name, display name, and tooltip
 *
 * @example
 * const { displayName, tooltipText } = useInterfaceName("vtnet0");
 * // displayName: "LAN"
 * // tooltipText: "LAN (vtnet0)"
 */
export function useInterfaceName(physicalName: string): InterfaceNameResult {
  const getLogicalName = useEnrichmentStore((state) => state.getLogicalName);

  const logicalName = getLogicalName(physicalName);

  // Display name: Logical name if available, otherwise physical name
  const displayName = logicalName || physicalName;

  // Tooltip text: Show mapping if available
  const tooltipText = logicalName
    ? `${logicalName} (${physicalName})`
    : `${physicalName} (no logical name configured)`;

  return {
    logicalName,
    physicalName,
    displayName,
    tooltipText,
  };
}
