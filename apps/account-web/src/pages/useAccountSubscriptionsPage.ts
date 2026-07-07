import { useSuspenseQuery } from '@tanstack/react-query';
import { accountBillingClient } from '@/account.billing.client';
import { useAccountContext } from '@/hooks/useAccountContext';

export function useAccountSubscriptionsWorkspaceId() {
  const { me } = useAccountContext();
  return me?.current_workspace_id ?? null;
}

export function useAccountSubscriptionsOverview(workspaceId: string) {
  const overviewQuery = useSuspenseQuery({
    queryKey: ['account-billing', 'overview', workspaceId],
    queryFn: () => accountBillingClient.getOverview(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });
  const portalQuery = useSuspenseQuery({
    queryKey: ['account-billing', 'portal-view', workspaceId],
    queryFn: () => accountBillingClient.getPortalView(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const openPortal = async () => {
    const portal = await accountBillingClient.createPortalSession(workspaceId);
    window.location.href = portal.url;
  };

  return {
    overview: overviewQuery.data,
    portal: portalQuery.data,
    openPortal,
  };
}
