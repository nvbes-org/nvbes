import type { BillingOverview } from '@nvbes/identity-client';
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

export function AccountSubscriptionsPlanOverview({ overview }: { overview: BillingOverview }) {
  const { plan, subscription } = overview;
  const status = subscription.status;

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
        <SubscriptionInfoRow label="Formule" value={plan.code} icon={Package} />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Prix mensuel"
          value={formatSubscriptionCents(plan.monthly_price_cents, plan.currency)}
          icon={CreditCard}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Stockage inclus"
          value={`${plan.included_storage_gb} Go`}
          icon={Zap}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Utilisateurs inclus"
          value={`${plan.included_users}`}
          icon={Zap}
        />
      </CardContent>
    </Card>
  );
}
