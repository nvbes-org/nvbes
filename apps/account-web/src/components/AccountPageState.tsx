import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';

export function AccountPageLoading({ label = 'Chargement…' }: { label?: string }) {
  return (
    <div className="flex min-h-40 items-center justify-center gap-2 text-sm text-muted-foreground">
      <Spinner />
      {label}
    </div>
  );
}

export function AccountPageError({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  return (
    <Alert variant="destructive">
      <AlertTitle>Impossible de charger ces informations</AlertTitle>
      <AlertDescription className="flex flex-col gap-4">
        <span>
          {error instanceof Error ? error.message : 'Une erreur inattendue est survenue.'}
        </span>
        <Button type="button" variant="outline" className="self-start" onClick={onRetry}>
          Réessayer
        </Button>
      </AlertDescription>
    </Alert>
  );
}
