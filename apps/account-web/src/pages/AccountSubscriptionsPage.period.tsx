import type { AccountBillingOverview } from '@/account.billing.client';
import { Package, Receipt, Zap } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { SubscriptionInfoRow } from '@/pages/AccountSubscriptionsPage.row';
import { formatSubscriptionDate } from '@/pages/AccountSubscriptionsPage.utils';

export function AccountSubscriptionsPeriodCard({ overview }: { overview: AccountBillingOverview }) {
  return (
    <Card className="animate-fade-slide-up [animation-delay:100ms]">
      <CardHeader>
        <CardTitle>Periode en cours</CardTitle>
        <CardDescription>Details de votre periode de facturation et estimation.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <SubscriptionInfoRow
          label="Debut de periode"
          value={formatSubscriptionDate(overview.current_period_start)}
          icon={Package}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Fin de periode"
          value={formatSubscriptionDate(overview.current_period_end)}
          icon={Package}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Retention incluse"
          value={`${overview.entitlements.retention_days} jours`}
          icon={Zap}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Audit"
          value={overview.entitlements.audit_level}
          icon={Receipt}
        />
      </CardContent>
    </Card>
  );
}
