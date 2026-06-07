import { ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function AccountSubscriptionsPortalCard({ onOpen }: { onOpen: () => Promise<void> }) {
  return (
    <Card className="animate-fade-slide-up [animation-delay:200ms]">
      <CardHeader>
        <CardTitle>Portail de facturation</CardTitle>
        <CardDescription>
          Accedez a votre portail Stripe pour consulter vos factures, modifier votre moyen de
          paiement et gerer votre abonnement.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="outline" className="w-full justify-between" onClick={() => void onOpen()}>
          Ouvrir le portail de facturation
          <ExternalLink className="size-4" data-icon="inline-end" />
        </Button>
      </CardContent>
    </Card>
  );
}
