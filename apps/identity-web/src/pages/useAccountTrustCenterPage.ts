import { identityClient } from '@nvbes/identity-client';
import { useSuspenseQuery } from '@tanstack/react-query';

export function useAccountTrustCenterPage() {
  return useSuspenseQuery({
    queryKey: ['identity', 'enterprise', 'trust-center'] as const,
    queryFn: ({ signal }) => identityClient.getEnterpriseTrustCenter({ signal }),
    staleTime: 60 * 1000,
    gcTime: 30 * 60 * 1000,
  });
}
