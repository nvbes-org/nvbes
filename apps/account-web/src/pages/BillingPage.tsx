import { useSuspenseQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { accountBillingClient } from '@/account.billing.client';
import { useAccountContext } from '@/hooks/useAccountContext';
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
    queryKey: ['account-billing', 'overview', workspaceId],
    queryFn: () => accountBillingClient.getOverview(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const handleCheckout = async (planCode: string) => {
    if (!workspaceId) return;
    setCheckoutLoading(planCode);
    try {
      const checkout = await accountBillingClient.createCheckoutSession(workspaceId, planCode);
      window.location.href = checkout.url;
    } finally {
      setCheckoutLoading(null);
    }
  };

  const handlePortal = async () => {
    if (!workspaceId) return;
    setPortalLoading(true);
    try {
      const portal = await accountBillingClient.createPortalSession(workspaceId);
      window.location.href = portal.url;
    } finally {
      setPortalLoading(false);
    }
  };

  const currentPlanCode = overview.plan_code;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <BillingPageIntro />

      <CurrentPlanCard overview={overview} />

      <BillingPortalCard
        provider={overview.billing_provider}
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
