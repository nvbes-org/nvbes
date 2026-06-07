import { type AccountSession, identityClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef, useState } from 'react';

import { accountQueryKeys } from '@/account.queries';

export function useAccountSessionsPage() {
  const [revoking, setRevoking] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const { data: sessions = [], isPending } = useQuery({
    queryKey: accountQueryKeys.sessions,
    queryFn: ({ signal }) => identityClient.listSessions({ signal }),
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const handleRevoke = async (sessionId: string) => {
    setRevoking(sessionId);
    const previous = queryClient.getQueryData<AccountSession[]>(accountQueryKeys.sessions) ?? [];
    queryClient.setQueryData<AccountSession[]>(
      accountQueryKeys.sessions,
      previous.filter((session) => session.id !== sessionId),
    );

    try {
      await identityClient.revokeSession(sessionId);
      await queryClient.invalidateQueries({ queryKey: ['account', 'security-overview'] });
    } catch {
      queryClient.setQueryData(accountQueryKeys.sessions, previous);
    } finally {
      setRevoking(null);
    }
  };

  const handleRevokeOthers = async () => {
    await identityClient.revokeOtherSessions();
    await queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
    await queryClient.invalidateQueries({ queryKey: ['account', 'security-overview'] });
  };

  const currentSession = sessions.find((session) => session.current);
  const otherSessions = sessions.filter((session) => !session.current);
  const rowCount = otherSessions.length > 0 ? otherSessions.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 8,
  });

  return {
    currentSession,
    isPending,
    listRef,
    otherSessions,
    revoking,
    sessions,
    totalSize: virtualizer.getTotalSize(),
    virtualItems: virtualizer.getVirtualItems(),
    virtualizer,
    onRevoke: handleRevoke,
    onRevokeOthers: handleRevokeOthers,
  };
}
