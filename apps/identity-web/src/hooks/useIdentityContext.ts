import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';

import { identifyUser } from '@/identity.analytics';
import { identityContextQueryOptions } from '@/identity.queries';
import type { AccountMe } from '@/lib/identity-session';

type IdentityContextState = {
  me: AccountMe | null;
  loading: boolean;
  error: Error | null;
  retrying: boolean;
  retry: () => void;
};

export function useIdentityContext(): IdentityContextState {
  const { data, error, isFetching, isPending, refetch } = useQuery(identityContextQueryOptions());

  useEffect(() => {
    if (!data) {
      return;
    }

    identifyUser(data.user.id, {
      country: data.user.region ?? 'unknown',
      status: 'authenticated',
    });
  }, [data]);

  return {
    me: data ?? null,
    loading: isPending,
    error,
    retrying: isFetching && !isPending,
    retry: () => {
      void refetch();
    },
  };
}
