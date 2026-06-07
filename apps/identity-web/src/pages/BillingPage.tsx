import { identityClient } from '@nvbes/identity-client';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { trackEvent } from '../identity.posthog';
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
      trackEvent('billing.checkout_started', {
        plan_code: planCode,
        workspace_id: workspaceId,
      });
      window.location.href = await identityClient.createBillingCheckout(workspaceId, planCode);
    } finally {
      setCheckoutLoading(null);
    }
  };

  const handlePortal = async () => {
    if (!workspaceId) return;
    setPortalLoading(true);
    try {
      window.location.href = await identityClient.createBillingPortal(workspaceId);
    } finally {
      setPortalLoading(false);
    }
  };

  const currentPlanCode = overview.plan.code;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <BillingPageIntro />

      <CurrentPlanCard overview={overview} />

      <BillingPortalCard portalLoading={portalLoading} onPortal={() => void handlePortal()} />

      <BillingPlansList
        currentPlanCode={currentPlanCode}
        checkoutLoading={checkoutLoading}
        onCheckout={(planCode) => void handleCheckout(planCode)}
      />
    </div>
  );
}
