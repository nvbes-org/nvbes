import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef, useState } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { listLinkedApps, revokeLinkedApp, type LinkedApp } from '@/pages/AccountLinkedAppsPage.api';

export function useAccountLinkedAppsPage() {
  const [revoking, setRevoking] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const { data: clients = [], isPending } = useQuery({
    queryKey: accountQueryKeys.linkedApps,
    queryFn: ({ signal }) => listLinkedApps(signal),
    staleTime: 5 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const rowCount = clients.length > 0 ? clients.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 64),
    overscan: 8,
  });

  const handleRevoke = async (clientId: string) => {
    const previous = queryClient.getQueryData<LinkedApp[]>(accountQueryKeys.linkedApps) ?? [];
    queryClient.setQueryData<LinkedApp[]>(
      accountQueryKeys.linkedApps,
      previous.filter((client) => client.id !== clientId),
    );
    setRevoking(clientId);
    try {
      await revokeLinkedApp(clientId);
    } catch {
      queryClient.setQueryData(accountQueryKeys.linkedApps, previous);
    } finally {
      setRevoking(null);
    }
  };

  return {
    clients,
    isPending,
    listRef,
    revoking,
    virtualizer,
    handleRevoke,
  };
}

export type UseAccountLinkedAppsPageResult = ReturnType<typeof useAccountLinkedAppsPage>;
