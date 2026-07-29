import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { accountContextQueryOptions } from '@/account.queries';
import { useAuthuser } from '@/hooks/useAuthuser';
import { identifyUser, setAnalyticsWorkspaceGroup } from '@/identity.analytics';
import type { AccountEntry, AccountMe } from '@/lib/account-context';

type AccountContextState = {
  me: AccountMe | null;
  accounts: AccountEntry[];
  loading: boolean;
  error: Error | null;
  retrying: boolean;
  retry: () => void;
};

export function useAccountContext() {
  const authuser = useAuthuser();
  const { data, error, isFetching, isPending, refetch } = useQuery(
    accountContextQueryOptions(authuser),
  );

  useEffect(() => {
    const me = data?.me;
    if (!me) {
      return;
    }

    identifyUser(me.user.id, {
      country: me.user.region ?? 'unknown',
      status: 'authenticated',
    });
    if (me.current_workspace_id) {
      void setAnalyticsWorkspaceGroup(me.current_workspace_id);
    }
  }, [data?.me]);

  return {
    me: data?.me ?? null,
    accounts: data?.accounts ?? [],
    loading: isPending,
    error,
    retrying: isFetching && !isPending,
    retry: () => {
      void refetch();
    },
  } satisfies AccountContextState;
}
