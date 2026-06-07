import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { useAccountContext } from '@/hooks/useAccountContext';
import { listLinkedIdentities, type LinkedIdentity, unlinkIdentity } from './AccountSocialPage.api';

export function useAccountSocialPage() {
  const [unlinking, setUnlinking] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { me, loading: accountLoading } = useAccountContext();
  const tenantId = me?.current_tenant_id ?? null;

  const { data: identities = [], isPending } = useQuery({
    queryKey: [...accountQueryKeys.all, 'linked-identities', tenantId] as const,
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

    const queryKey = [...accountQueryKeys.all, 'linked-identities', tenantId] as const;
    const previous = queryClient.getQueryData<LinkedIdentity[]>(queryKey) ?? [];

    queryClient.setQueryData<LinkedIdentity[]>(
      queryKey,
      previous.filter((entry) => entry.id !== identity.id),
    );
    setUnlinking(identity.id);

    try {
      await unlinkIdentity(tenantId, identity.id);
    } catch {
      queryClient.setQueryData(queryKey, previous);
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
