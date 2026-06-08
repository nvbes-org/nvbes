import { CreditCard, Database, Users2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import type { DriveBillingState, DriveWorkspaceState } from './drive.workspace.types';

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

const BYTE_UNITS = ['o', 'Ko', 'Mo', 'Go', 'To'];

export function DriveBillingView({ state }: { state: DriveWorkspaceState }) {
  const billing = state.billing;
  const storagePercent = storageUsagePercent(billing);
  const nextInvoiceAmount = invoiceAmountForPlan(billing.plan);

  return (
    <section className="grid gap-4">
      <div className="grid gap-3 lg:grid-cols-3">
        <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
          <CreditCard className="mb-4 size-5 text-primary" aria-hidden="true" />
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">Plan</p>
          <h2 className="mt-1 text-2xl font-semibold">{planLabel(billing.plan)}</h2>
          <p className="mt-1 text-sm text-muted-foreground">{billingStatusLabel(billing.status)}</p>
        </article>

        <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
          <Users2 className="mb-4 size-5 text-primary" aria-hidden="true" />
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">Seats</p>
          <h2 className="mt-1 text-2xl font-semibold">
            {billing.seatsUsed}/{billing.seatsIncluded}
          </h2>
          <p className="mt-1 text-sm text-muted-foreground">Sieges utilises dans le workspace.</p>
        </article>

        <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
          <Database className="mb-4 size-5 text-primary" aria-hidden="true" />
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
            Prochaine facture
          </p>
          <h2 className="mt-1 text-2xl font-semibold">{nextInvoiceAmount}</h2>
          <p className="mt-1 text-sm text-muted-foreground">Le {formatDate(billing.renewalDate)}</p>
        </article>
      </div>

      <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <div className="min-w-0">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
              Stockage
            </p>
            <h2 className="mt-1 text-lg font-semibold">
              {formatBytes(billing.storageUsedBytes)} utilises sur {formatBytes(billing.storageLimitBytes)}
            </h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Essai: {trialEndLabel(billing)}. Renouvellement le {formatDate(billing.renewalDate)}.
            </p>
          </div>
          <Button type="button" variant="outline" disabled title="Portail Stripe non connecte dans cette vue.">
            Portail facture indisponible
          </Button>
        </div>
        <Progress className="mt-4" value={storagePercent} aria-label="Utilisation du stockage" />
        <p className="mt-2 text-sm text-muted-foreground">{storagePercent}% du quota utilise.</p>
      </article>
    </section>
  );
}

function storageUsagePercent(billing: DriveBillingState): number {
  if (billing.storageLimitBytes <= 0) {
    return 0;
  }

  return Math.min(100, Math.round((billing.storageUsedBytes / billing.storageLimitBytes) * 100));
}

function formatBytes(value: number): string {
  let unitIndex = 0;
  let amount = value;

  while (amount >= 1024 && unitIndex < BYTE_UNITS.length - 1) {
    amount /= 1024;
    unitIndex += 1;
  }

  return `${amount.toLocaleString('fr-FR', { maximumFractionDigits: amount >= 10 ? 0 : 1 })} ${BYTE_UNITS[unitIndex]}`;
}

function formatDate(value: string): string {
  return DATE_FORMATTER.format(new Date(value));
}

function planLabel(plan: DriveBillingState['plan']): string {
  if (plan === 'enterprise') return 'Enterprise';
  if (plan === 'team') return 'Team';
  return 'Free';
}

function billingStatusLabel(status: DriveBillingState['status']): string {
  if (status === 'trialing') return 'Essai actif';
  if (status === 'active') return 'Abonnement actif';
  if (status === 'past_due') return 'Paiement en retard';
  return 'Annule';
}

function invoiceAmountForPlan(plan: DriveBillingState['plan']): string {
  if (plan === 'enterprise') return 'Sur devis';
  if (plan === 'team') return '96 EUR';
  return '0 EUR';
}

function trialEndLabel(billing: DriveBillingState): string {
  if (billing.status === 'trialing') {
    return formatDate(billing.renewalDate);
  }

  return 'aucun essai actif';
}
