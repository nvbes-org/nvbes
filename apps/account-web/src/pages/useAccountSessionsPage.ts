import { type AccountSession, identityClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import { accountQueryKeys } from '@/account.queries';
import { useAuthuser } from '@/hooks/useAuthuser';
import { isStepUpRequiredError } from '@/identity.step-up';
import { type DeviceGroup, groupSessionsByDevice } from './AccountSessionsPage.device';

export function useAccountSessionsPage() {
  const authuser = useAuthuser();
  const [revoking, setRevoking] = useState<string | null>(null);
  const [confirmingRisk, setConfirmingRisk] = useState(false);
  const [showRiskStepUp, setShowRiskStepUp] = useState(false);
  const [showRevokeOthersStepUp, setShowRevokeOthersStepUp] = useState(false);
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
    try {
      await identityClient.revokeOtherSessions();
      setShowRevokeOthersStepUp(false);
      await queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
      await queryClient.invalidateQueries({ queryKey: securityOverviewQueryKey });
    } catch (error) {
      if (isStepUpRequiredError(error)) {
        setShowRevokeOthersStepUp(true);
        return;
      }
      throw error;
    }
  };

  const handleConfirmRisk = async () => {
    const current = sessions.find((session) => session.current);
    if (!current) return;
    setConfirmingRisk(true);
    try {
      await identityClient.confirmHighRiskSession(current.id);
      await queryClient.invalidateQueries({ queryKey: sessionsQueryKey });
      setShowRiskStepUp(false);
    } finally {
      setConfirmingRisk(false);
    }
  };

  const { currentDevice, recognizedDevices, otherDevices } = groupSessionsByDevice(sessions);
  const currentSession = sessions.find((session) => session.current);

  return {
    currentDevice,
    currentSession,
    confirmingRisk,
    isPending,
    otherDevices,
    recognizedDevices,
    revoking,
    sessions,
    onRevoke: handleRevoke,
    onRevokeDevice: handleRevokeDevice,
    onRevokeOthers: handleRevokeOthers,
    onRequestRiskConfirmation: () => setShowRiskStepUp(true),
    onRiskStepUpSuccess: handleConfirmRisk,
    onCancelRiskConfirmation: () => setShowRiskStepUp(false),
    onCancelRevokeOthers: () => setShowRevokeOthersStepUp(false),
    onRevokeOthersStepUpSuccess: handleRevokeOthers,
    showRevokeOthersStepUp,
    showRiskStepUp,
  };
}
