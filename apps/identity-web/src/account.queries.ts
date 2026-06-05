import { identityClient } from '@nvbes/identity-client';
import { queryOptions } from '@tanstack/react-query';

export const accountQueryKeys = {
  all: ['identity', 'account'] as const,
  context: ['identity', 'account', 'context'] as const,
  linkedApps: ['identity', 'account', 'linked-apps'] as const,
  personalInfo: ['identity', 'account', 'personal-info'] as const,
  sessions: ['identity', 'account', 'sessions'] as const,
};

export const accountContextQueryOptions = queryOptions({
  queryKey: accountQueryKeys.context,
  queryFn: async ({ signal }) => {
    const [meResult, workspaces, accounts] = await Promise.allSettled([
      identityClient.getMe({ signal }),
      identityClient.listWorkspaces({ signal }),
      identityClient.listAccounts({ signal }),
    ]);

    return {
      me: meResult.status === 'fulfilled' ? meResult.value : null,
      workspaces: workspaces.status === 'fulfilled' ? workspaces.value : [],
      accounts: accounts.status === 'fulfilled' ? accounts.value : [],
    };
  },
  staleTime: 30 * 1000,
  gcTime: 30 * 60 * 1000,
  refetchOnWindowFocus: true,
});
