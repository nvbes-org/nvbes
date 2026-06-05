import { type BillingOverview, identityClient } from '@nvbes/identity-client';
import { useSuspenseQuery } from '@tanstack/react-query';
import { CreditCard, ExternalLink, Package, Receipt, Zap } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { useAccountContext } from '@/hooks/useAccountContext';

type SubscriptionStatus = 'active' | 'trialing' | 'past_due' | 'canceled' | 'incomplete' | string;

const statusLabels: Record<string, string> = {
  active: 'Actif',
  trialing: 'Essai',
  past_due: 'En retard',
  canceled: 'Annule',
  incomplete: 'Incomplet',
};

const statusVariants: Record<string, 'default' | 'secondary' | 'destructive' | 'outline'> = {
  active: 'default',
  trialing: 'secondary',
  past_due: 'destructive',
  canceled: 'outline',
  incomplete: 'outline',
};

function formatCents(cents: number, currency: string): string {
  return new Intl.NumberFormat('fr-FR', {
    style: 'currency',
    currency: currency.toUpperCase(),
  }).format(cents / 100);
}

function formatDate(dateStr: string | null | undefined): string {
  if (!dateStr) return '—';
  return new Date(dateStr).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

function InfoRow({
  label,
  value,
  icon: Icon,
}: {
  label: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
}) {
  return (
    <div className="flex items-center gap-3 py-1">
      <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
        <Icon className="size-4 text-muted-foreground" />
      </div>
      <div className="flex flex-col min-w-0">
        <span className="text-xs text-muted-foreground">{label}</span>
        <span className="text-sm font-medium truncate">{value}</span>
      </div>
    </div>
  );
}

function PlanOverview({ overview }: { overview: BillingOverview }) {
  const { plan, subscription } = overview;
  const status = subscription.status as SubscriptionStatus;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Abonnement actuel</CardTitle>
            <CardDescription>Details de votre formule et statut de facturation.</CardDescription>
          </div>
          <Badge variant={statusVariants[status] ?? 'secondary'}>
            {statusLabels[status] ?? status}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <InfoRow label="Formule" value={plan.code} icon={Package} />
        <Separator className="my-1" />
        <InfoRow
          label="Prix mensuel"
          value={formatCents(plan.monthly_price_cents, plan.currency)}
          icon={CreditCard}
        />
        <Separator className="my-1" />
        <InfoRow label="Stockage inclus" value={`${plan.included_storage_gb} Go`} icon={Zap} />
        <Separator className="my-1" />
        <InfoRow label="Utilisateurs inclus" value={`${plan.included_users}`} icon={Zap} />
      </CardContent>
    </Card>
  );
}

function SubscriptionDetails({ overview }: { overview: BillingOverview }) {
  const { subscription, invoice_estimate } = overview;

  return (
    <Card className="animate-fade-slide-up [animation-delay:100ms]">
      <CardHeader>
        <CardTitle>Periode en cours</CardTitle>
        <CardDescription>Details de votre periode de facturation et estimation.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <InfoRow
          label="Debut de periode"
          value={formatDate(subscription.current_period_start)}
          icon={Package}
        />
        <Separator className="my-1" />
        <InfoRow
          label="Fin de periode"
          value={formatDate(subscription.current_period_end)}
          icon={Package}
        />
        {subscription.trial_ends_at && (
          <>
            <Separator className="my-1" />
            <InfoRow
              label="Fin de l'essai"
              value={formatDate(subscription.trial_ends_at)}
              icon={Zap}
            />
          </>
        )}
        <Separator className="my-1" />
        <InfoRow
          label="Estimation facture"
          value={formatCents(invoice_estimate.estimated_amount_cents, invoice_estimate.currency)}
          icon={Receipt}
        />
      </CardContent>
    </Card>
  );
}

export default function AccountSubscriptionsPage() {
  const { me } = useAccountContext();
  const workspaceId = me?.current_workspace_id;

  const handlePortal = async () => {
    if (!workspaceId) return;
    window.location.href = await identityClient.createBillingPortal(workspaceId);
  };

  if (!workspaceId) {
    return (
      <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
        <div>
          <h1 className="text-xl font-heading font-semibold">Abonnements</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Gerer vos abonnements, factures et historique de paiement.
          </p>
        </div>
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-8">
            <Package className="size-8 text-muted-foreground" />
            <div className="text-center">
              <p className="text-sm text-muted-foreground">
                Vous devez avoir un workspace actif pour consulter vos abonnements.
              </p>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return <SubscriptionsContent workspaceId={workspaceId} handlePortal={handlePortal} />;
}

function SubscriptionsContent({
  workspaceId,
  handlePortal,
}: {
  workspaceId: string;
  handlePortal: () => Promise<void>;
}) {
  const { data: overview } = useSuspenseQuery({
    queryKey: ['billing', 'overview', workspaceId],
    queryFn: () => identityClient.getBillingOverview(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Abonnements</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer vos abonnements, factures et historique de paiement.
        </p>
      </div>

      <PlanOverview overview={overview} />
      <SubscriptionDetails overview={overview} />

      <Card className="animate-fade-slide-up [animation-delay:200ms]">
        <CardHeader>
          <CardTitle>Portail de facturation</CardTitle>
          <CardDescription>
            Accedez a votre portail Stripe pour consulter vos factures, modifier votre moyen de
            paiement et gerer votre abonnement.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="outline" className="w-full justify-between" onClick={handlePortal}>
            Ouvrir le portail de facturation
            <ExternalLink className="size-4" data-icon="inline-end" />
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
