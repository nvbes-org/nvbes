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
      const me = await identityClient.getMe({ signal });
      const accounts = await identityClient.listAccounts({ signal }).catch(() => []);

      return {
        me,
        accounts,
      };
    },
    staleTime: 30 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });
}
