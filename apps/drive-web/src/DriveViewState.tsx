import { AlertTriangle, Loader2 } from 'lucide-react';
import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function DriveEmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <Card className="border-dashed bg-background/80 shadow-sm">
      <CardHeader className="items-center text-center">
        <div className="mb-2 grid size-12 place-items-center rounded-2xl bg-muted">
          <span className="text-lg" aria-hidden="true">
            //
          </span>
        </div>
        <CardTitle>{title}</CardTitle>
        <CardDescription className="max-w-md">{description}</CardDescription>
      </CardHeader>
      {action ? <CardContent className="flex justify-center">{action}</CardContent> : null}
    </Card>
  );
}

export function DriveErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <Card className="border-destructive/30 bg-destructive/5 shadow-sm">
      <CardHeader className="items-center text-center">
        <div className="mb-2 grid size-12 place-items-center rounded-2xl bg-destructive/10 text-destructive">
          <AlertTriangle className="size-5" aria-hidden="true" />
        </div>
        <CardTitle>Impossible de charger Drive</CardTitle>
        <CardDescription className="max-w-md">{message}</CardDescription>
      </CardHeader>
      {onRetry ? (
        <CardContent className="flex justify-center">
          <Button type="button" variant="outline" onClick={onRetry}>
            Reessayer
          </Button>
        </CardContent>
      ) : null}
    </Card>
  );
}

export function DriveLoadingState({ label = 'Chargement de Drive...' }: { label?: string }) {
  return (
    <Card className="bg-background/80 shadow-sm">
      <CardContent className="flex items-center justify-center gap-3 p-8 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" aria-hidden="true" />
        {label}
      </CardContent>
    </Card>
  );
}
