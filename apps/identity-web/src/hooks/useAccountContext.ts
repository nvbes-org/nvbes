import { useQuery } from '@tanstack/react-query';
import { accountContextQueryOptions } from '@/account.queries';
import type { AccountEntry, AccountMe, AccountWorkspace } from '@/lib/account-context';

type AccountContextState = {
  me: AccountMe | null;
  workspaces: AccountWorkspace[];
  accounts: AccountEntry[];
  loading: boolean;
};

export function useAccountContext() {
  const { data, isPending } = useQuery(accountContextQueryOptions);

  return {
    me: data?.me ?? null,
    workspaces: data?.workspaces ?? [],
    accounts: data?.accounts ?? [],
    loading: isPending,
  } satisfies AccountContextState;
}
