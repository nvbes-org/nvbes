import { Loader2Icon } from 'lucide-react';
import { ResultCard } from './VerifyEmailResultPage.card';

export function VerifyEmailVerifyingState() {
  return (
    <ResultCard
      title="Vérification en cours"
      description="Nous vérifions ton adresse email..."
      icon={<Loader2Icon className="size-10 animate-spin text-muted-foreground" />}
    />
  );
}
