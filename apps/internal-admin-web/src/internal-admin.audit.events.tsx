import { useQuery } from '@tanstack/react-query';
import { RefreshCw, ShieldCheck } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { listAuditEvents } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, AuditEvent } from './internal-admin.types';

export function AuditEventsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const auditEvents = useQuery({
    queryKey: ['internal-audit-events', credentials.workspaceId],
    queryFn: () => listAuditEvents(credentials, 25),
    enabled: !disabled,
  });

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="audit">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="bg-primary/10 text-primary flex size-9 items-center justify-center rounded-md">
            <ShieldCheck className="size-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold">Audit recent</h2>
            <p className="text-muted-foreground text-xs">
              Derniers evenements append-only du tenant courant.
            </p>
          </div>
        </div>
        <Button
          disabled={disabled || auditEvents.isFetching}
          onClick={() => void auditEvents.refetch()}
          size="sm"
          type="button"
          variant="outline"
        >
          <RefreshCw className="size-4" />
          Refresh
        </Button>
      </div>

      <div className="space-y-2">
        {disabled ? (
          <LockedState label="Connecte un contexte operateur pour consulter l'audit." />
        ) : null}
        {auditEvents.isLoading ? <Skeleton className="h-20 w-full" /> : null}
        {(auditEvents.data ?? []).map((event) => (
          <AuditEventRow event={event} key={event.id} />
        ))}
        {auditEvents.data?.length === 0 ? (
          <p className="text-muted-foreground rounded-md border p-3 text-sm">
            Aucun evenement audit trouve pour ce tenant.
          </p>
        ) : null}
        {auditEvents.error ? (
          <p className="text-destructive rounded-md border p-3 text-sm">
            {auditEvents.error instanceof Error ? auditEvents.error.message : 'Audit unavailable'}
          </p>
        ) : null}
      </div>
    </section>
  );
}

function AuditEventRow({ event }: { event: AuditEvent }) {
  return (
    <article className="bg-muted/40 rounded-md p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <h3 className="truncate text-sm font-medium">{event.action}</h3>
          <p className="text-muted-foreground mt-1 truncate font-mono text-xs">
            {event.target_type}
            {event.target_id ? `:${event.target_id}` : ''}
          </p>
        </div>
        <Badge variant="secondary">{formatDate(event.created_at)}</Badge>
      </div>
      <div className="text-muted-foreground mt-2 grid gap-1 text-xs md:grid-cols-2">
        <span className="truncate">
          Actor: {event.actor_email ?? event.actor_principal_id ?? 'system'}
        </span>
        <span className="truncate">Event: {event.id}</span>
      </div>
    </article>
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('fr-FR', {
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    month: '2-digit',
  });
}
