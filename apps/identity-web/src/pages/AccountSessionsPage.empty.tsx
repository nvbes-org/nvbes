import { Laptop } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';

export function EmptySessionsCard() {
  return (
    <Card>
      <CardContent className="flex flex-col items-center gap-3 py-12">
        <div className="flex size-12 items-center justify-center rounded-full bg-muted">
          <Laptop className="size-6 text-muted-foreground" />
        </div>
        <div className="text-center">
          <p className="text-sm font-medium">Aucune session</p>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Impossible de charger vos sessions.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
