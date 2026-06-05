import { useEffect, useState } from 'react';
import { apiMonitor, type ConnectionStatus } from '../lib/api-monitor';

export function useApiStatus(): {
  status: ConnectionStatus;
  checkConnection: () => Promise<boolean>;
} {
  const [status, setStatus] = useState<ConnectionStatus>(() => apiMonitor.getStatus());

  useEffect(() => {
    return apiMonitor.subscribe((newStatus) => {
      setStatus(newStatus);
    });
  }, []);

  return {
    status,
    checkConnection: () => apiMonitor.checkConnectionNow(),
  };
}
