import { AlertTriangle } from 'lucide-react';
import type { ReactNode } from 'react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { Spinner } from '@/components/ui/spinner';

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
    <Empty className="border border-dashed">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <span aria-hidden="true">//</span>
        </EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
      {action ? <EmptyContent>{action}</EmptyContent> : null}
    </Empty>
  );
}

export function DriveErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <Alert variant="destructive">
      <AlertTriangle />
      <AlertTitle>Impossible de charger Drive</AlertTitle>
      <AlertDescription>{message}</AlertDescription>
      {onRetry ? (
        <div className="mt-3">
          <Button type="button" variant="outline" onClick={onRetry}>
            Reessayer
          </Button>
        </div>
      ) : null}
    </Alert>
  );
}

export function DriveLoadingState({ label = 'Chargement de Drive...' }: { label?: string }) {
  return (
    <Card className="bg-background/80 shadow-sm">
      <CardContent className="flex items-center justify-center gap-3 p-8 text-sm text-muted-foreground">
        <Spinner />
        {label}
      </CardContent>
    </Card>
  );
}
