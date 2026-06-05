import { useVirtualizer } from '@tanstack/react-virtual';
import { AlertTriangle, FileSearch, Shield } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { z } from 'zod';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';
import { identityHttpClient } from '../identity.http';

const SecurityEventSchema = z.object({
  id: z.string(),
  event_type: z.string(),
  created_at: z.string(),
  ip_address: z.string().nullable().optional(),
  user_agent: z.string().nullable().optional(),
  status: z.string().optional(),
});

const SecurityEventsResponseSchema = z.object({
  events: z.array(SecurityEventSchema),
  next_cursor: z.string().nullable().optional(),
});

type SecurityEvent = z.infer<typeof SecurityEventSchema>;

function listSecurityEvents(workspaceId: string, limit = 20) {
  return identityHttpClient.get(
    `/workspaces/${workspaceId}/security-events?limit=${limit}`,
    SecurityEventsResponseSchema,
  );
}

const eventLabels: Record<string, string> = {
  login_success: 'Connexion reussie',
  login_failed: 'Echec de connexion',
  password_changed: 'Mot de passe modifie',
  password_reset: 'Reinitialisation du mot de passe',
  mfa_enrolled: 'MFA activee',
  mfa_removed: 'MFA desactivee',
  session_revoked: 'Session revoquee',
  consent_granted: 'Consentement accorde',
  consent_revoked: 'Consentement revoque',
  account_created: 'Compte cree',
  email_verified: 'Email verifie',
  export_requested: 'Export demande',
  account_deleted: 'Compte supprime',
};

function AuditsSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-40" />
        <Skeleton className="h-4 w-72 mt-1" />
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-32" />
          <Skeleton className="h-4 w-48" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export default function AccountAuditsPage() {
  const [events, setEvents] = useState<SecurityEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const { me } = useAccountContext();
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const fetchEvents = async () => {
      if (me?.current_workspace_id) {
        try {
          const result = await listSecurityEvents(me.current_workspace_id, 50);
          setEvents(result.events);
        } catch {
          /* user may not have permission */
        }
      }
      setLoading(false);
    };
    fetchEvents();
  }, [me?.current_workspace_id]);

  const rowCount = events.length > 0 ? events.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 10,
  });

  if (loading) return <AuditsSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Audits & RGPD</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Journaux d&apos;audit, rapports de conformite et exports RGPD.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Journal de securite</CardTitle>
          <CardDescription>
            Evenements de securite recents pour votre workspace actuel.
            {events.length > 0 && ` (${events.length} evenements)`}
          </CardDescription>
        </CardHeader>
        <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
          {events.length === 0 ? (
            <div className="px-6 py-8">
              <div className="flex flex-col items-center gap-3">
                <FileSearch className="size-8 text-muted-foreground" />
                <div className="text-center">
                  <p className="text-sm text-muted-foreground">
                    {me?.current_workspace_id
                      ? 'Aucun evenement de securite disponible.'
                      : 'Selectionnez un workspace pour voir les evenements de securite.'}
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

                const event = events[Math.floor(virtualItem.index / 2)];
                const label = eventLabels[event.event_type] ?? event.event_type;
                const date = new Date(event.created_at).toLocaleDateString('fr-FR', {
                  year: 'numeric',
                  month: 'short',
                  day: 'numeric',
                  hour: '2-digit',
                  minute: '2-digit',
                });

                return (
                  <div
                    key={event.id}
                    ref={virtualizer.measureElement}
                    data-index={virtualItem.index}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <div className="flex items-center justify-between gap-3 py-1">
                      <div className="flex min-w-0 items-center gap-3">
                        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                          {event.event_type.includes('failed') ||
                          event.event_type.includes('error') ? (
                            <AlertTriangle className="size-4 text-destructive/70" />
                          ) : (
                            <Shield className="size-4 text-muted-foreground" />
                          )}
                        </div>
                        <div className="flex min-w-0 flex-col">
                          <span className="text-sm font-medium">{label}</span>
                          <span className="text-xs text-muted-foreground">
                            {date}
                            {event.ip_address ? ` · ${event.ip_address}` : ''}
                          </span>
                        </div>
                      </div>
                      <Badge variant="secondary" className="shrink-0">
                        {event.status ?? 'success'}
                      </Badge>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="animate-fade-slide-up [animation-delay:100ms]">
        <CardHeader>
          <CardTitle>Exports RGPD</CardTitle>
          <CardDescription>Conformite RGPD et demandes d&apos;acces aux donnees.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <Button
            variant="outline"
            className="w-full justify-between"
            onClick={() => window.open('/account/privacy', '_self')}
          >
            Demander l&apos;export de mes donnees
            <FileSearch className="size-4" data-icon="inline-end" />
          </Button>
          <p className="text-xs text-muted-foreground">
            Conformement au RGPD, vous pouvez demander une copie de toutes vos donnees personnelles
            ou la suppression complete de votre compte.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
