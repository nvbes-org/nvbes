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
  const portalQuery = useSuspenseQuery({
    queryKey: ['billing', 'portal-view', workspaceId],
    queryFn: () => identityClient.getBillingPortalView(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const openPortal = async () => {
    const portal = await identityClient.createBillingPortalSession(workspaceId);
    window.location.href = portal.url;
  };

  return {
    overview: overviewQuery.data,
    portal: portalQuery.data,
    openPortal,
  };
}
