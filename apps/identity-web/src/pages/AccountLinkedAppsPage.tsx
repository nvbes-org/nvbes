import { identityClient, type OAuthClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { ExternalLink, Unlink, X } from 'lucide-react';
import { useRef, useState } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';

function LinkedAppsSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-32" />
        <Skeleton className="h-4 w-64 mt-1" />
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-32" />
          <Skeleton className="h-4 w-48" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

function listLinkedApps(signal?: AbortSignal): Promise<OAuthClient[]> {
  return identityClient.listOAuthClients({ signal });
}

export default function AccountLinkedAppsPage() {
  const [revoking, setRevoking] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const { data: clients = [], isPending } = useQuery({
    queryKey: accountQueryKeys.linkedApps,
    queryFn: ({ signal }) => listLinkedApps(signal),
    staleTime: 5 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const rowCount = clients.length > 0 ? clients.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 64),
    overscan: 8,
  });

  const handleRevoke = async (clientId: string) => {
    const previous = queryClient.getQueryData<OAuthClient[]>(accountQueryKeys.linkedApps) ?? [];
    queryClient.setQueryData<OAuthClient[]>(
      accountQueryKeys.linkedApps,
      previous.filter((client) => client.id !== clientId),
    );
    setRevoking(clientId);
    try {
      await identityClient.revokeOAuthClient(clientId);
    } catch {
      queryClient.setQueryData(accountQueryKeys.linkedApps, previous);
    } finally {
      setRevoking(null);
    }
  };

  if (isPending) return <LinkedAppsSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Apps liees</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer les applications et services connectes a votre compte.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Applications OAuth</CardTitle>
          <CardDescription>
            Applications tierces autorisees a acceder a votre compte. Revoguez l&apos;acces pour les
            applications que vous n&apos;utilisez plus.
          </CardDescription>
        </CardHeader>
        <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
          {clients.length === 0 ? (
            <div className="px-6 py-8">
              <div className="flex flex-col items-center gap-3">
                <Unlink className="size-8 text-muted-foreground" />
                <div className="text-center">
                  <p className="text-sm text-muted-foreground">
                    Aucune application tierce autorisee.
                  </p>
                </div>
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

                const client = clients[Math.floor(virtualItem.index / 2)];

                return (
                  <div
                    key={client.id}
                    ref={virtualizer.measureElement}
                    data-index={virtualItem.index}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <div className="flex items-center justify-between gap-3 py-1">
                      <div className="flex min-w-0 items-center gap-3">
                        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                          <ExternalLink className="size-4 text-muted-foreground" />
                        </div>
                        <div className="flex min-w-0 flex-col">
                          <span className="truncate text-sm font-medium">{client.name}</span>
                          <span className="text-xs text-muted-foreground">
                            {client.client_type} · Cree le {formatDate(client.created_at)}
                          </span>
                        </div>
                      </div>
                      <div className="flex shrink-0 items-center gap-2">
                        <Badge variant="secondary">{client.client_type}</Badge>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                          onClick={() => handleRevoke(client.id)}
                          disabled={revoking === client.id}
                          aria-label={`Revoquer ${client.name}`}
                        >
                          {revoking === client.id ? (
                            <span className="size-3 animate-spin rounded-full border-2 border-current border-t-transparent" />
                          ) : (
                            <X className="size-3.5" />
                          )}
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
