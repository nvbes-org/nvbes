import {
  type AccountEntry,
  type AccountMe,
  type AccountWorkspace,
  identityClient,
} from '@nvbes/identity-client';
import { useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Building, Loader2, Plus } from 'lucide-react';
import { useRef, useState } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';

function WorkspacesSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-40" />
        <Skeleton className="h-4 w-64 mt-1" />
      </div>
      <Card>
        <CardContent className="flex flex-col gap-3 pt-6">
          {Array.from({ length: 2 }).map((_, i) => (
            <Skeleton key={i} className="h-14 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

export default function WorkspacesPage() {
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const { me, workspaces, loading } = useAccountContext();
  const queryClient = useQueryClient();
  const listRef = useRef<HTMLDivElement>(null);

  const rowCount = workspaces.length > 0 ? workspaces.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 8,
  });

  const handleCreate = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setCreateError(null);
    if (!newWorkspaceName.trim()) {
      setCreateError('Le nom du workspace est requis.');
      return;
    }

    setCreating(true);
    try {
      const ws = await identityClient.createWorkspace({ name: newWorkspaceName.trim() });
      queryClient.setQueryData<{
        me: AccountMe | null;
        workspaces: AccountWorkspace[];
        accounts: AccountEntry[];
      }>(accountQueryKeys.context, (current) =>
        current
          ? {
              ...current,
              workspaces: [...current.workspaces, ws],
            }
          : current,
      );
      setNewWorkspaceName('');
      setShowCreateDialog(false);
    } catch {
      setCreateError('Impossible de creer le workspace. Veuillez reessayer.');
    } finally {
      setCreating(false);
    }
  };

  const handleOpenCreate = () => {
    setNewWorkspaceName('');
    setCreateError(null);
    setShowCreateDialog(true);
  };

  if (loading) return <WorkspacesSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-xl font-heading font-semibold">Workspaces</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Gerer vos espaces de travail et organisations.
          </p>
        </div>
        <Button variant="outline" size="sm" className="shrink-0" onClick={handleOpenCreate}>
          <Plus className="size-4" data-icon="inline-start" />
          Creer
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Mes workspaces</CardTitle>
          <CardDescription>
            {workspaces.length} workspace{workspaces.length !== 1 ? 's' : ''} disponible
            {workspaces.length !== 1 ? 's' : ''}.
          </CardDescription>
        </CardHeader>
        <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
          {workspaces.length === 0 ? (
            <div className="px-6 py-8">
              <div className="flex flex-col items-center gap-3">
                <Building className="size-8 text-muted-foreground" />
                <p className="text-sm text-muted-foreground">Aucun workspace trouve.</p>
                <Button variant="outline" size="sm" onClick={handleOpenCreate}>
                  <Plus className="size-3.5" data-icon="inline-start" />
                  Creer un workspace
                </Button>
              </div>
            </div>
          ) : (
            <div
              style={{
                height: `${virtualizer.getTotalSize()}px`,
                position: 'relative',
              }}
            >
              {virtualizer.getVirtualItems().map((virtualItem) => {
                if (virtualItem.index % 2 === 1) {
                  return (
                    <div
                      key={`separator-${virtualItem.index}`}
                      className="absolute left-0 right-0 px-6"
                      style={{ transform: `translateY(${virtualItem.start}px)` }}
                    >
                      <Separator className="my-1" />
                    </div>
                  );
                }

                const ws = workspaces[Math.floor(virtualItem.index / 2)];

                return (
                  <div
                    key={ws.id}
                    ref={virtualizer.measureElement}
                    data-index={virtualItem.index}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <div className="flex items-center justify-between gap-3 py-1">
                      <div className="flex min-w-0 items-center gap-3">
                        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                          <Building className="size-4 text-muted-foreground" />
                        </div>
                        <div className="flex flex-col min-w-0">
                          <span className="text-sm font-medium">{ws.name}</span>
                          <span className="text-xs text-muted-foreground">
                            {ws.workspace_type} · {ws.role}
                            {ws.plan_code ? ` · ${ws.plan_code}` : ''}
                          </span>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {ws.id === me?.current_workspace_id && (
                          <Badge variant="default" className="shrink-0">
                            Actuel
                          </Badge>
                        )}
                        <Badge variant="secondary" className="shrink-0">
                          {ws.role}
                        </Badge>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Creer un workspace</DialogTitle>
            <DialogDescription>
              Creez un nouvel espace de travail pour votre equipe ou organisation.
            </DialogDescription>
          </DialogHeader>
          <form onSubmit={handleCreate}>
            <div className="flex flex-col gap-4 py-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="workspace-name">Nom du workspace</Label>
                <Input
                  id="workspace-name"
                  type="text"
                  placeholder="Mon entreprise"
                  value={newWorkspaceName}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                    setNewWorkspaceName(e.target.value)
                  }
                  required
                  autoFocus
                  maxLength={100}
                />
                <p className="text-xs text-muted-foreground">
                  Le nom de votre organisation ou de votre projet.
                </p>
              </div>
              {createError && <ErrorMessage message={createError} />}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setShowCreateDialog(false)}
                disabled={creating}
              >
                Annuler
              </Button>
              <Button type="submit" disabled={creating}>
                {creating ? (
                  <>
                    <Loader2 className="size-4 animate-spin" data-icon="inline-start" />
                    Creation...
                  </>
                ) : (
                  'Creer'
                )}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
