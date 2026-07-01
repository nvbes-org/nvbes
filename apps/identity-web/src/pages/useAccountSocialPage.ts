import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation } from '@tanstack/react-router';
import { useState } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { useAccountContext } from '@/hooks/useAccountContext';
import { readAuthuser } from '@/identity.authuser';
import { listLinkedIdentities, type LinkedIdentity, unlinkIdentity } from './AccountSocialPage.api';

export function useAccountSocialPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
  const [unlinking, setUnlinking] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { me, loading: accountLoading } = useAccountContext();
  const tenantId = me?.current_tenant_id ?? null;
  const linkedIdentitiesQueryKey = accountQueryKeys.linkedIdentities(authuser, tenantId);

  const { data: identities = [], isPending } = useQuery({
    queryKey: linkedIdentitiesQueryKey,
    queryFn: ({ signal }) =>
      tenantId ? listLinkedIdentities(tenantId, signal) : Promise.resolve([]),
    enabled: Boolean(tenantId),
    staleTime: 5 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const handleUnlink = async (identity: LinkedIdentity) => {
    if (!tenantId) {
      return;
    }

    const previous = queryClient.getQueryData<LinkedIdentity[]>(linkedIdentitiesQueryKey) ?? [];

    queryClient.setQueryData<LinkedIdentity[]>(
      linkedIdentitiesQueryKey,
      previous.filter((entry) => entry.id !== identity.id),
    );
    setUnlinking(identity.id);

    try {
      await unlinkIdentity(tenantId, identity.id);
    } catch {
      queryClient.setQueryData(linkedIdentitiesQueryKey, previous);
    } finally {
      setUnlinking(null);
    }
  };

  return {
    accountLoading,
    identities,
    isPending,
    me,
    tenantId,
    unlinking,
    handleUnlink,
  };
}
