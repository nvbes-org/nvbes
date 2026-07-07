import type { AccountBillingOverview } from '@/account.billing.client';
import { Check, CreditCard, Package } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { availablePlans } from './BillingPage.plans.data';

export function CurrentPlanCard({ overview }: { overview: AccountBillingOverview }) {
  const plan = availablePlans.find((candidate) => candidate.code === overview.plan_code);
  const price = plan ? `${plan.price}€` : overview.plan_code;

  return (
    <Card className="ring-2 ring-primary/30">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>{plan?.name ?? overview.plan_code}</CardTitle>
          <Badge variant="default" className="shrink-0">
            Plan actuel
          </Badge>
        </div>
        <CardDescription>
          <span className="text-2xl font-bold text-foreground">{price}</span>
          <span className="text-sm text-muted-foreground">/mois</span>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <Check className="size-3.5 shrink-0 text-primary" />
            <span className="text-sm text-muted-foreground">
              {overview.entitlements.included_users} utilisateur
              {overview.entitlements.included_users > 1 ? 's' : ''}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-3.5 shrink-0 text-primary" />
            <span className="text-sm text-muted-foreground">
              {overview.entitlements.included_storage_gb} Go de stockage
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-3.5 shrink-0 text-primary" />
            <span className="text-sm text-muted-foreground">
              Statut : {overview.subscription_status}
            </span>
          </div>
        </div>
      </CardContent>
      <CardFooter>
        <Button variant="outline" className="w-full justify-between" disabled>
          Abonnement actif
          <Package className="size-4" data-icon="inline-end" />
        </Button>
      </CardFooter>
    </Card>
  );
}

export function BillingPortalCard({
  provider,
  portalLoading,
  onPortal,
}: {
  provider: string;
  portalLoading: boolean;
  onPortal: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Portail de facturation</CardTitle>
        <CardDescription>
          Accedez au portail de paiement {provider} pour gerer vos moyens de paiement et factures.
        </CardDescription>
      </CardHeader>
      <CardFooter>
        <Button
          variant="outline"
          className="w-full justify-between"
          onClick={onPortal}
          disabled={portalLoading}
        >
          {portalLoading ? 'Chargement...' : 'Ouvrir le portail de facturation'}
          <CreditCard data-icon="inline-end" />
        </Button>
      </CardFooter>
    </Card>
  );
}
