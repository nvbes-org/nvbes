import type { AccountBillingPortalView } from '@/account.billing.client';
import { CreditCard, ExternalLink, FileText, Route, ShieldCheck, WalletCards } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { SubscriptionInfoRow } from '@/pages/AccountSubscriptionsPage.row';

function paymentMethodLabel(method: AccountBillingPortalView['payment_methods'][number]): string {
  const cardLabel = [method.brand, method.last4 ? `**** ${method.last4}` : null]
    .filter(Boolean)
    .join(' ');
  return cardLabel || method.payment_method_id;
}

function invoiceAmount(invoice: AccountBillingPortalView['invoices'][number]): string {
  return new Intl.NumberFormat('fr-FR', {
    style: 'currency',
    currency: invoice.currency,
  }).format(invoice.total_minor / 100);
}

function invoiceLabel(invoice: AccountBillingPortalView['invoices'][number]): string {
  return invoice.invoice_number ?? invoice.invoice_id;
}

function subscriptionRole(subscription: AccountBillingPortalView['subscriptions'][number]): string {
  if (subscription.providers.some((provider) => provider.primary)) {
    return 'Primaire';
  }
  if (subscription.providers.some((provider) => provider.fallback_eligible)) {
    return 'Backup';
  }
  return 'Secondaire';
}

export function AccountSubscriptionsPortalCard({
  portal,
  onOpen,
}: {
  portal: AccountBillingPortalView;
  onOpen: () => Promise<void>;
}) {
  return (
    <Card className="animate-fade-slide-up [animation-delay:200ms]">
      <CardHeader>
        <CardTitle>Portail de facturation</CardTitle>
        <CardDescription>
          Accedez a votre portail de paiement pour consulter vos factures, modifier votre moyen de
          paiement et gerer votre abonnement.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <SubscriptionInfoRow
          label="Prestataire actif"
          value={portal.provider || 'Non configure'}
          icon={CreditCard}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Routage abonnement"
          value={`${portal.subscriptions.length} lien${portal.subscriptions.length > 1 ? 's' : ''}`}
          icon={Route}
        />
        {portal.subscriptions.map((subscription, index) => (
          <div
            key={`${subscription.subscription_id}-${subscription.status}-${index}`}
            className="flex items-center justify-between gap-3 rounded-md border border-border/60 px-3 py-2 text-sm"
          >
            <div className="min-w-0">
              <p className="truncate font-medium">
                {subscription.providers.find((provider) => provider.primary)?.provider ??
                  subscription.providers[0]?.provider ??
                  subscription.subscription_id}
              </p>
              <p className="truncate text-muted-foreground">{subscription.status}</p>
            </div>
            <span className="shrink-0 text-muted-foreground">{subscriptionRole(subscription)}</span>
          </div>
        ))}
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Mise a jour du moyen de paiement"
          value="Portail prestataire"
          icon={WalletCards}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Identifiants prestataire exposes"
          value="Non"
          icon={ShieldCheck}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Factures disponibles"
          value={`${portal.invoices.length}`}
          icon={FileText}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Moyens de paiement"
          value={`${portal.payment_methods.length}`}
          icon={WalletCards}
        />
        {portal.invoices.map((invoice) => {
          return (
            <div
              key={invoice.invoice_id}
              className="flex items-center justify-between gap-3 rounded-md border border-border/60 px-3 py-2 text-sm"
            >
              <div className="min-w-0">
                <p className="truncate font-medium">{invoiceLabel(invoice)}</p>
                <p className="truncate text-muted-foreground">
                  {invoice.status} - {invoiceAmount(invoice)}
                  {invoice.providers[0]?.provider ? ` - ${invoice.providers[0].provider}` : ''}
                </p>
              </div>
            </div>
          );
        })}
        {portal.payment_methods.map((method) => (
          <div
            key={method.payment_method_id}
            className="flex items-center justify-between gap-3 rounded-md border border-border/60 px-3 py-2 text-sm"
          >
            <div className="min-w-0">
              <p className="truncate font-medium">{paymentMethodLabel(method)}</p>
              <p className="truncate text-muted-foreground">
                {method.providers
                  .map((provider) => `${provider.provider} - ${provider.status}`)
                  .join(', ') || 'Aucun prestataire actif'}
              </p>
            </div>
            <span className="shrink-0 text-muted-foreground">
              {method.exp_month && method.exp_year
                ? `${method.exp_month.toString().padStart(2, '0')}/${method.exp_year}`
                : 'Actif'}
            </span>
          </div>
        ))}
        <Separator className="my-3" />
        <Button variant="outline" className="w-full justify-between" onClick={() => void onOpen()}>
          Ouvrir le portail de facturation
          <ExternalLink className="size-4" data-icon="inline-end" />
        </Button>
      </CardContent>
    </Card>
  );
}
