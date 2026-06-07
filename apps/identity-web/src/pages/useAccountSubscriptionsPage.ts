import { identityClient } from '@nvbes/identity-client';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useAccountContext } from '@/hooks/useAccountContext';

export function useAccountSubscriptionsWorkspaceId() {
  const { me } = useAccountContext();
  return me?.current_workspace_id ?? null;
}

export function useAccountSubscriptionsOverview(workspaceId: string) {
  const overviewQuery = useSuspenseQuery({
    queryKey: ['billing', 'overview', workspaceId],
    queryFn: () => identityClient.getBillingOverview(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const openPortal = async () => {
    window.location.href = await identityClient.createBillingPortal(workspaceId);
  };

  return {
    overview: overviewQuery.data,
    openPortal,
  };
}
