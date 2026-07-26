import type { ReactNode, SubmitEvent } from 'react';
import { Card } from '@/components/ui/card';

export function StepUpFormHeader({ description }: { description?: string }) {
  return (
    <div className="flex flex-col gap-1.5 text-center">
      <h1 className="mt-2 text-xl font-heading font-semibold">Vérification requise</h1>
      <p className="text-xs text-muted-foreground">
        {description || 'Veuillez confirmer votre identité pour continuer.'}
      </p>
    </div>
  );
}

export function StepUpFormCard({
  children,
  onSubmit,
}: {
  children: ReactNode;
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
}) {
  return (
    <form onSubmit={onSubmit} className="w-full max-w-md">
      <Card className="space-y-4 p-6">{children}</Card>
    </form>
  );
}
