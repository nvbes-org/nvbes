import type { AccountBillingOverview } from '@/account.billing.client';
import { CreditCard, Package, Zap } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { SubscriptionInfoRow } from '@/pages/AccountSubscriptionsPage.row';
import {
  formatSubscriptionCents,
  subscriptionStatusLabels,
  subscriptionStatusVariants,
} from '@/pages/AccountSubscriptionsPage.utils';
import { availablePlans } from './BillingPage.plans.data';

export function AccountSubscriptionsPlanOverview({
  overview,
}: {
  overview: AccountBillingOverview;
}) {
  const plan = availablePlans.find((candidate) => candidate.code === overview.plan_code);
  const status = overview.subscription_status;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Abonnement actuel</CardTitle>
            <CardDescription>Details de votre formule et statut de facturation.</CardDescription>
          </div>
          <Badge variant={subscriptionStatusVariants[status] ?? 'secondary'}>
            {subscriptionStatusLabels[status] ?? status}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <SubscriptionInfoRow
          label="Formule"
          value={plan?.name ?? overview.plan_code}
          icon={Package}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Prix mensuel"
          value={plan ? formatSubscriptionCents(Number(plan.price) * 100, 'eur') : 'Non disponible'}
          icon={CreditCard}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Stockage inclus"
          value={`${overview.entitlements.included_storage_gb} Go`}
          icon={Zap}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Utilisateurs inclus"
          value={`${overview.entitlements.included_users}`}
          icon={Zap}
        />
      </CardContent>
    </Card>
  );
}
