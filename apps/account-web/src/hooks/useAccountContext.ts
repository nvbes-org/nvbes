import { useQuery } from '@tanstack/react-query';
import { useLocation } from '@tanstack/react-router';
import { accountContextQueryOptions } from '@/account.queries';
import { readAuthuser } from '@/identity.authuser';
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
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr, location.pathname);
  const { data, error, isFetching, isPending, refetch } = useQuery(
    accountContextQueryOptions(authuser),
  );

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
