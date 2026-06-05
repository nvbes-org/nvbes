import { type AccountSession, identityClient } from '@nvbes/identity-client';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Laptop, Smartphone, X } from 'lucide-react';
import { useRef, useState } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';

function SessionsSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-48" />
        <Skeleton className="h-4 w-72 mt-1" />
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

function parseUserAgent(ua: string): { browser: string; os: string } {
  let browser = 'Navigateur inconnu';
  let os = 'OS inconnu';

  if (ua.includes('Firefox')) browser = 'Firefox';
  else if (ua.includes('Edg')) browser = 'Edge';
  else if (ua.includes('Chrome')) browser = 'Chrome';
  else if (ua.includes('Safari')) browser = 'Safari';

  if (ua.includes('Windows')) os = 'Windows';
  else if (ua.includes('Mac OS') || ua.includes('Macintosh')) os = 'macOS';
  else if (ua.includes('Linux')) os = 'Linux';
  else if (ua.includes('Android')) os = 'Android';
  else if (ua.includes('iPhone') || ua.includes('iPad')) os = 'iOS';

  return { browser, os };
}

export default function AccountSessionsPage() {
  const [revoking, setRevoking] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const { data: sessions = [], isPending } = useQuery({
    queryKey: accountQueryKeys.sessions,
    queryFn: ({ signal }) => identityClient.listSessions({ signal }),
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const handleRevoke = async (sessionId: string) => {
    setRevoking(sessionId);
    const previous = queryClient.getQueryData<AccountSession[]>(accountQueryKeys.sessions) ?? [];
    queryClient.setQueryData<AccountSession[]>(
      accountQueryKeys.sessions,
      previous.filter((session) => session.id !== sessionId),
    );
    try {
      await identityClient.revokeSession(sessionId);
      await queryClient.invalidateQueries({ queryKey: ['account', 'security-overview'] });
    } catch {
      queryClient.setQueryData(accountQueryKeys.sessions, previous);
    } finally {
      setRevoking(null);
    }
  };

  const handleRevokeOthers = async () => {
    await identityClient.revokeOtherSessions();
    await queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
    await queryClient.invalidateQueries({ queryKey: ['account', 'security-overview'] });
  };

  const currentSession = sessions.find((s) => s.current);
  const otherSessions = sessions.filter((s) => !s.current);
  const rowCount = otherSessions.length > 0 ? otherSessions.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 8,
  });

  if (isPending) return <SessionsSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Appareils & sessions</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer vos sessions actives sur tous vos appareils.
        </p>
      </div>

      {currentSession && (
        <Card>
          <CardHeader>
            <CardTitle>Session actuelle</CardTitle>
            <CardDescription>L&apos;appareil que vous utilisez actuellement.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                <Laptop className="size-4 text-primary" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">
                  {currentSession.user_agent
                    ? parseUserAgent(currentSession.user_agent).browser
                    : 'Appareil inconnu'}
                </span>
                <span className="text-xs text-muted-foreground">
                  {currentSession.user_agent ? parseUserAgent(currentSession.user_agent).os : ''}
                  {currentSession.ip ? ` · ${currentSession.ip}` : ''}
                </span>
              </div>
              <Badge variant="default" className="shrink-0 ml-auto">
                Actuel
              </Badge>
            </div>
          </CardContent>
        </Card>
      )}

      {otherSessions.length > 0 && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>Autres sessions</CardTitle>
                <CardDescription>
                  {otherSessions.length} session(s) active(s) sur d&apos;autres appareils.
                </CardDescription>
              </div>
              <Button variant="outline" size="sm" onClick={handleRevokeOthers} className="shrink-0">
                Deconnecter les autres
              </Button>
            </div>
          </CardHeader>
          <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
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

                const session = otherSessions[Math.floor(virtualItem.index / 2)];
                const { browser, os } = session.user_agent
                  ? parseUserAgent(session.user_agent)
                  : { browser: 'Appareil inconnu', os: '' };

                return (
                  <div
                    key={session.id}
                    ref={virtualizer.measureElement}
                    data-index={virtualItem.index}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <div className="flex items-center gap-3 py-1">
                      <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                        <Smartphone className="size-4 text-muted-foreground" />
                      </div>
                      <div className="flex min-w-0 flex-1 flex-col">
                        <span className="text-sm font-medium">{browser}</span>
                        <span className="text-xs text-muted-foreground">
                          {os}
                          {session.ip ? ` · ${session.ip}` : ''}
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        disabled={revoking === session.id}
                        onClick={() => handleRevoke(session.id)}
                      >
                        <X className="size-4" />
                        <span className="sr-only">Revoquer</span>
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      )}

      {sessions.length === 0 && (
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-12">
            <div className="flex size-12 items-center justify-center rounded-full bg-muted">
              <Laptop className="size-6 text-muted-foreground" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium">Aucune session</p>
              <p className="text-xs text-muted-foreground mt-0.5">
                Impossible de charger vos sessions.
              </p>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
