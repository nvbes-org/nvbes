import { Plus } from 'lucide-react';

import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';

export function WorkspacesSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-40" />
        <Skeleton className="mt-1 h-4 w-64" />
      </div>
      <Card>
        <CardContent className="flex flex-col gap-3 pt-6">
          {Array.from({ length: 2 }).map((_, index) => (
            <Skeleton key={index} className="h-14 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export function WorkspacesError({ message }: { message: string }) {
  return (
    <Alert variant="destructive">
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

export function WorkspacesHeader({ onOpenCreate }: { onOpenCreate: () => void }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div>
        <h1 className="text-xl font-heading font-semibold">Workspaces</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Gerer vos espaces de travail et organisations.
        </p>
      </div>
      <Button variant="outline" size="sm" className="shrink-0" onClick={onOpenCreate}>
        <Plus className="size-4" data-icon="inline-start" />
        Creer
      </Button>
    </div>
  );
}
