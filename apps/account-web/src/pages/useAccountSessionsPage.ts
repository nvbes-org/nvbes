import { type AccountSession, identityClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation } from '@tanstack/react-router';
import { useState } from 'react';

import { accountQueryKeys } from '@/account.queries';
import { readAuthuser } from '@/identity.authuser';
import { trackEvent } from '@/identity.analytics';
import { type DeviceGroup, groupSessionsByDevice } from './AccountSessionsPage.device';

export function useAccountSessionsPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr, location.pathname);
  const [revoking, setRevoking] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const sessionsQueryKey = accountQueryKeys.sessions(authuser);
  const securityOverviewQueryKey = accountQueryKeys.securityOverview(authuser);

  const { data: sessions = [], isPending } = useQuery({
    queryKey: sessionsQueryKey,
    queryFn: async ({ signal }) => {
      const sessions: AccountSession[] = [];
      let cursor: string | undefined;
      do {
        const page = await identityClient.listSessionsPage({ limit: 200, cursor, signal });
        sessions.push(...page.sessions);
        cursor = page.has_more && page.next_cursor ? page.next_cursor : undefined;
      } while (cursor);
      return sessions;
    },
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
      trackEvent('account.session_revoked');
      await queryClient.invalidateQueries({ queryKey: securityOverviewQueryKey });
    } catch {
      queryClient.setQueryData(sessionsQueryKey, previous);
    } finally {
      setRevoking(null);
    }
  };

  const handleRevokeDevice = async (device: DeviceGroup) => {
    const toRevoke = device.sessions.filter((s) => !s.current);
    for (const session of toRevoke) {
      await handleRevoke(session.id);
    }
  };

  const handleRevokeOthers = async () => {
    await identityClient.revokeOtherSessions();
    trackEvent('account.all_other_sessions_revoked');
    await queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
    await queryClient.invalidateQueries({ queryKey: securityOverviewQueryKey });
  };

  const { currentDevice, recognizedDevices, otherDevices } = groupSessionsByDevice(sessions);
  const currentSession = sessions.find((session) => session.current);

  return {
    currentDevice,
    currentSession,
    isPending,
    otherDevices,
    recognizedDevices,
    revoking,
    sessions,
    onRevoke: handleRevoke,
    onRevokeDevice: handleRevokeDevice,
    onRevokeOthers: handleRevokeOthers,
  };
}
