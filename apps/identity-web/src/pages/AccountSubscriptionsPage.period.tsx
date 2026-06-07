import type { BillingOverview } from '@nvbes/identity-client';
import { Package, Receipt, Zap } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { SubscriptionInfoRow } from '@/pages/AccountSubscriptionsPage.row';
import {
  formatSubscriptionCents,
  formatSubscriptionDate,
} from '@/pages/AccountSubscriptionsPage.utils';

export function AccountSubscriptionsPeriodCard({ overview }: { overview: BillingOverview }) {
  const { subscription, invoice_estimate } = overview;

  return (
    <Card className="animate-fade-slide-up [animation-delay:100ms]">
      <CardHeader>
        <CardTitle>Periode en cours</CardTitle>
        <CardDescription>Details de votre periode de facturation et estimation.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <SubscriptionInfoRow
          label="Debut de periode"
          value={formatSubscriptionDate(subscription.current_period_start)}
          icon={Package}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Fin de periode"
          value={formatSubscriptionDate(subscription.current_period_end)}
          icon={Package}
        />
        {subscription.trial_ends_at ? (
          <>
            <Separator className="my-1" />
            <SubscriptionInfoRow
              label="Fin de l'essai"
              value={formatSubscriptionDate(subscription.trial_ends_at)}
              icon={Zap}
            />
          </>
        ) : null}
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Estimation facture"
          value={formatSubscriptionCents(
            invoice_estimate.estimated_amount_cents,
            invoice_estimate.currency,
          )}
          icon={Receipt}
        />
      </CardContent>
    </Card>
  );
}
