import { type AccountSession, identityClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation } from '@tanstack/react-router';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef, useState } from 'react';

import { accountQueryKeys } from '@/account.queries';
import { readAuthuser } from '@/identity.authuser';

export function useAccountSessionsPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
  const [revoking, setRevoking] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();
  const sessionsQueryKey = accountQueryKeys.sessions(authuser);
  const securityOverviewQueryKey = accountQueryKeys.securityOverview(authuser);

  const { data: sessions = [], isPending } = useQuery({
    queryKey: sessionsQueryKey,
    queryFn: ({ signal }) => identityClient.listSessions({ signal }),
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const handleRevoke = async (sessionId: string) => {
    setRevoking(sessionId);
    const previous = queryClient.getQueryData<AccountSession[]>(sessionsQueryKey) ?? [];
    queryClient.setQueryData<AccountSession[]>(
      sessionsQueryKey,
      previous.filter((session) => session.id !== sessionId),
    );

    try {
      await identityClient.revokeSession(sessionId);
      await queryClient.invalidateQueries({ queryKey: securityOverviewQueryKey });
    } catch {
      queryClient.setQueryData(sessionsQueryKey, previous);
    } finally {
      setRevoking(null);
    }
  };

  const handleRevokeOthers = async () => {
    await identityClient.revokeOtherSessions();
    await queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
    await queryClient.invalidateQueries({ queryKey: securityOverviewQueryKey });
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
