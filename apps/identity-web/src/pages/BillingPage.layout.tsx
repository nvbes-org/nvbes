import { CreditCard } from 'lucide-react';

import { Card, CardContent } from '@/components/ui/card';

export function BillingPageIntro() {
  return (
    <div>
      <h1 className="text-xl font-heading font-semibold">Facturation</h1>
      <p className="mt-1 text-sm text-muted-foreground">
        Gerer votre abonnement et votre portefeuille.
      </p>
    </div>
  );
}

export function BillingWorkspaceRequiredCard() {
  return (
    <Card>
      <CardContent className="flex flex-col items-center gap-3 py-8">
        <CreditCard className="size-8 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">
          Vous devez avoir un workspace actif pour acceder a la facturation.
        </p>
      </CardContent>
    </Card>
  );
}
