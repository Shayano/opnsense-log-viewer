import { useEnrichmentStore } from '@/stores/enrichment-store';

interface IPAliasResult {
  aliases: string[];
  displayText: string;
  tooltipText: string;
}

/**
 * Hook to resolve IP aliases (IP → alias names with group members)
 *
 * @param ip - IP address (e.g., "192.168.1.100")
 * @returns IPAliasResult with aliases, display text, and tooltip
 *
 * @example
 * const { displayText, tooltipText } = useIPAlias("192.168.1.100");
 * // displayText: "192.168.1.100 (Servers_Group, DMZ_Hosts)"
 * // tooltipText: "Servers_Group: 192.168.1.100, 192.168.1.101, 192.168.1.102\nDMZ_Hosts: 192.168.1.100, 10.0.1.5"
 */
export function useIPAlias(ip: string): IPAliasResult {
  const getAliasesForIP = useEnrichmentStore((state) => state.getAliasesForIP);

  const aliasData = getAliasesForIP(ip);

  const aliasNames = aliasData?.map((a) => a.aliasName) || [];

  // Display text: IP with alias names in parentheses
  const displayText =
    aliasNames.length > 0 ? `${ip} (${aliasNames.join(', ')})` : ip;

  // Tooltip text: Show group members for each alias
  const tooltipText =
    aliasData && aliasData.length > 0
      ? aliasData
          .map((alias) => `${alias.aliasName}: ${alias.groupMembers.join(', ')}`)
          .join('\n')
      : ip;

  return {
    aliases: aliasNames,
    displayText,
    tooltipText,
  };
}
