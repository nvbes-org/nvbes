import { Loader2Icon } from 'lucide-react';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function VerifyEmailVerifyingState() {
  return (
    <Card className="w-full max-w-sm">
      <CardHeader className="pb-2 text-center">
        <div className="mx-auto mb-3">
          <Loader2Icon className="size-10 animate-spin text-muted-foreground" />
        </div>
        <CardTitle>Vérification en cours</CardTitle>
        <CardDescription>Nous vérifions ton adresse email...</CardDescription>
      </CardHeader>
    </Card>
  );
}
