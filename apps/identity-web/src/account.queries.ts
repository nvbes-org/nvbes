import { identityClient } from '@nvbes/identity-client';
import { queryOptions } from '@tanstack/react-query';

export const accountQueryKeys = {
  all: ['identity', 'account'] as const,
  byAuthuser: (authuser: string) => ['identity', 'account', authuser] as const,
  context: (authuser: string) => [...accountQueryKeys.byAuthuser(authuser), 'context'] as const,
  emails: (authuser: string) => [...accountQueryKeys.byAuthuser(authuser), 'emails'] as const,
  linkedApps: (authuser: string) =>
    [...accountQueryKeys.byAuthuser(authuser), 'linked-apps'] as const,
  linkedIdentities: (authuser: string, tenantId: string | null) =>
    [...accountQueryKeys.byAuthuser(authuser), 'linked-identities', tenantId] as const,
  personalInfo: (authuser: string) =>
    [...accountQueryKeys.byAuthuser(authuser), 'personal-info'] as const,
  securityOverview: (authuser: string) =>
    [...accountQueryKeys.byAuthuser(authuser), 'security-overview'] as const,
  sessions: (authuser: string) => [...accountQueryKeys.byAuthuser(authuser), 'sessions'] as const,
};

export function accountContextQueryOptions(authuser: string) {
  return queryOptions({
    queryKey: accountQueryKeys.context(authuser),
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
}
