import {
  AccountSubscriptionsPageHeader,
  AccountSubscriptionsPeriodCard,
  AccountSubscriptionsPlanOverview,
  AccountSubscriptionsPortalCard,
  AccountSubscriptionsWorkspaceRequiredCard,
} from './AccountSubscriptionsPage.shared';
import {
  useAccountSubscriptionsOverview,
  useAccountSubscriptionsWorkspaceId,
} from './useAccountSubscriptionsPage';

export default function AccountSubscriptionsPage() {
  const workspaceId = useAccountSubscriptionsWorkspaceId();

  if (!workspaceId) {
    return (
      <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
        <AccountSubscriptionsPageHeader />
        <AccountSubscriptionsWorkspaceRequiredCard />
      </div>
    );
  }

  return <AccountSubscriptionsContent workspaceId={workspaceId} />;
}

function AccountSubscriptionsContent({ workspaceId }: { workspaceId: string }) {
  const { overview, portal, openPortal } = useAccountSubscriptionsOverview(workspaceId);

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <AccountSubscriptionsPageHeader />
      <AccountSubscriptionsPlanOverview overview={overview} />
      <AccountSubscriptionsPeriodCard overview={overview} />
      <AccountSubscriptionsPortalCard portal={portal} onOpen={openPortal} />
    </div>
  );
}
