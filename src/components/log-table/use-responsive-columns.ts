import { useState, useEffect } from 'react';

export interface ColumnVisibility {
  timestamp: boolean;
  interface: boolean;
  sourceIp: boolean;
  sourcePort: boolean;
  destinationIp: boolean;
  destinationPort: boolean;
  protocol: boolean;
  action: boolean;
  ruleLabel: boolean;
}

export function useResponsiveColumns(): ColumnVisibility {
  const [visibility, setVisibility] = useState<ColumnVisibility>({
    timestamp: true,
    interface: true,
    sourceIp: true,
    sourcePort: true,
    destinationIp: true,
    destinationPort: true,
    protocol: true,
    action: true,
    ruleLabel: true,
  });

  useEffect(() => {
    const handleResize = (): void => {
      const width = window.innerWidth;

      if (width < 1280) {
        // Hide less critical columns on small screens
        // Priority: Timestamp > Action > IPs > Ports > Interface > Protocol > Rule
        setVisibility({
          timestamp: true,
          interface: false,
          sourceIp: true,
          sourcePort: true,
          destinationIp: true,
          destinationPort: true,
          protocol: false,
          action: true,
          ruleLabel: false,
        });
      } else {
        // Show all columns on larger screens
        setVisibility({
          timestamp: true,
          interface: true,
          sourceIp: true,
          sourcePort: true,
          destinationIp: true,
          destinationPort: true,
          protocol: true,
          action: true,
          ruleLabel: true,
        });
      }
    };

    handleResize(); // Initial check
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  return visibility;
}
