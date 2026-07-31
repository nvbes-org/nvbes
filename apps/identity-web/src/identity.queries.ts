import { identityClient } from '@nvbes/identity-client';
import { queryOptions } from '@tanstack/react-query';

export const identityQueryKeys = {
  all: ['identity'] as const,
  context: ['identity', 'context'] as const,
  emails: ['identity', 'emails'] as const,
  linkedApps: ['identity', 'linked-apps'] as const,
  securityOverview: ['identity', 'security-overview'] as const,
  sessions: ['identity', 'sessions'] as const,
};

export function identityContextQueryOptions() {
  return queryOptions({
    queryKey: identityQueryKeys.context,
    queryFn: ({ signal }) => identityClient.getMe({ signal }),
    staleTime: 30 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });
}
