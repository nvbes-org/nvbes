import { Package } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';

export function AccountSubscriptionsWorkspaceRequiredCard() {
  return (
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
  );
}
