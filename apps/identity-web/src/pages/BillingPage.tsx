import { identityClient } from '@nvbes/identity-client';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { trackEvent } from '../identity.analytics';
import {
  BillingPageIntro,
  BillingPlansList,
  BillingPortalCard,
  BillingWorkspaceRequiredCard,
  CurrentPlanCard,
} from './BillingPage.shared';

export default function BillingPage() {
  const { me } = useAccountContext();
  const workspaceId = me?.current_workspace_id;

  if (!workspaceId) {
    return (
      <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
        <BillingPageIntro />
        <BillingWorkspaceRequiredCard />
      </div>
    );
  }

  return <BillingContent workspaceId={workspaceId} />;
}

function BillingContent({ workspaceId }: { workspaceId: string }) {
  const [checkoutLoading, setCheckoutLoading] = useState<string | null>(null);
  const [portalLoading, setPortalLoading] = useState(false);

  const { data: overview } = useSuspenseQuery({
    queryKey: ['billing', 'overview', workspaceId],
    queryFn: () => identityClient.getBillingOverview(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const handleCheckout = async (planCode: string) => {
    if (!workspaceId) return;
    setCheckoutLoading(planCode);
    try {
      const checkout = await identityClient.createBillingCheckoutSession(workspaceId, planCode);
      trackEvent('billing.checkout_started', {
        plan_code: planCode,
        provider: checkout.provider,
        provider_customer_id: checkout.provider_customer_id,
        provider_product_id: checkout.provider_product_id,
        provider_price_id: checkout.provider_price_id,
        workspace_id: workspaceId,
      });
      window.location.href = checkout.url;
    } finally {
      setCheckoutLoading(null);
    }
  };

  const handlePortal = async () => {
    if (!workspaceId) return;
    setPortalLoading(true);
    try {
      const portal = await identityClient.createBillingPortalSession(workspaceId);
      trackEvent('billing.portal_started', {
        provider: portal.provider,
        provider_customer_id: portal.provider_customer_id,
        workspace_id: workspaceId,
      });
      window.location.href = portal.url;
    } finally {
      setPortalLoading(false);
    }
  };

  const currentPlanCode = overview.plan.code;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <BillingPageIntro />

      <CurrentPlanCard overview={overview} />

      <BillingPortalCard
        provider={overview.subscription.billing_provider}
        portalLoading={portalLoading}
        onPortal={() => void handlePortal()}
      />

      <BillingPlansList
        currentPlanCode={currentPlanCode}
        checkoutLoading={checkoutLoading}
        onCheckout={(planCode) => void handleCheckout(planCode)}
      />
    </div>
  );
}
