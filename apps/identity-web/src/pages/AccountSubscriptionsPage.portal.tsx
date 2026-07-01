import type { BillingPortalView } from '@nvbes/identity-client';
import { CreditCard, ExternalLink, FileText, ShieldCheck, WalletCards } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { SubscriptionInfoRow } from '@/pages/AccountSubscriptionsPage.row';

export function AccountSubscriptionsPortalCard({
  portal,
  onOpen,
}: {
  portal: BillingPortalView;
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
        <SubscriptionInfoRow label="Prestataire actif" value={portal.provider} icon={CreditCard} />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Mise a jour du moyen de paiement"
          value={portal.payment_method_update_flow}
          icon={WalletCards}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Identifiants prestataire exposes"
          value={portal.exposes_provider_secret_ids ? 'Oui' : 'Non'}
          icon={ShieldCheck}
        />
        <Separator className="my-1" />
        <SubscriptionInfoRow
          label="Factures disponibles"
          value={`${portal.invoices.length}`}
          icon={FileText}
        />
        <Separator className="my-3" />
        <Button variant="outline" className="w-full justify-between" onClick={() => void onOpen()}>
          Ouvrir le portail de facturation
          <ExternalLink className="size-4" data-icon="inline-end" />
        </Button>
      </CardContent>
    </Card>
  );
}
