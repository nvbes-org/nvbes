import {
  type BillingOverview,
  type BillingProviderCode,
  type PaymentMethodUpdateFlow,
} from '@nvbes/billing-client';
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

function formatCents(cents: number, currency: string): string {
  return new Intl.NumberFormat('fr-FR', {
    style: 'currency',
    currency: currency.toUpperCase(),
  }).format(cents / 100);
}

export function CurrentPlanCard({ overview }: { overview: BillingOverview }) {
  const { plan, subscription } = overview;

  return (
    <Card className="ring-2 ring-primary/30">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>{plan.code}</CardTitle>
          <Badge variant="default" className="shrink-0">
            Plan actuel
          </Badge>
        </div>
        <CardDescription>
          <span className="text-2xl font-bold text-foreground">
            {formatCents(plan.monthly_price_cents, plan.currency)}
          </span>
          <span className="text-sm text-muted-foreground">/mois</span>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <Check className="size-3.5 shrink-0 text-primary" />
            <span className="text-sm text-muted-foreground">
              {plan.included_users} utilisateur{plan.included_users > 1 ? 's' : ''}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-3.5 shrink-0 text-primary" />
            <span className="text-sm text-muted-foreground">
              {plan.included_storage_gb} Go de stockage
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-3.5 shrink-0 text-primary" />
            <span className="text-sm text-muted-foreground">
              Statut : {subscription.status}
              {subscription.trial_ends_at && " (periode d'essai)"}
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
  paymentMethodUpdateFlow,
  provider,
  portalLoading,
  onPortal,
}: {
  paymentMethodUpdateFlow: PaymentMethodUpdateFlow;
  provider: BillingProviderCode;
  portalLoading: boolean;
  onPortal: () => void;
}) {
  const canOpenProviderPortal = paymentMethodUpdateFlow === 'nvbes_provider_redirect';

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
          disabled={portalLoading || !canOpenProviderPortal}
        >
          {portalLoading
            ? 'Chargement...'
            : canOpenProviderPortal
              ? 'Ouvrir le portail de facturation'
              : 'Portail prestataire indisponible'}
          <CreditCard data-icon="inline-end" />
        </Button>
      </CardFooter>
    </Card>
  );
}
