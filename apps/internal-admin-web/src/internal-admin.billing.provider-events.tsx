import { ClipboardButton } from '@nvbes/web-ui';
import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, RefreshCw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { listProviderEventFailures } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, ProviderEventFailure } from './internal-admin.types';

export function ProviderEventFailuresPanel({
  credentials,
  disabled,
  onReplay,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onReplay: (event: ProviderEventFailure) => void;
}) {
  const failures = useQuery({
    queryKey: ['provider-event-failures', credentials.workspaceId],
    queryFn: () => listProviderEventFailures(credentials),
    enabled: !disabled,
  });

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="provider-events">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="bg-destructive/10 text-destructive flex size-9 items-center justify-center rounded-md">
            <AlertTriangle className="size-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold">Provider event failures</h2>
            <p className="text-muted-foreground text-xs">
              Events failed/rejected a inspecter avant replay.
            </p>
          </div>
        </div>
        <Button
          disabled={disabled || failures.isFetching}
          onClick={() => void failures.refetch()}
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
          <LockedState label="Connecte un contexte operateur pour inspecter les provider events." />
        ) : null}
        {failures.isLoading ? <Skeleton className="h-24 w-full" /> : null}
        {(failures.data ?? []).map((failure) => (
          <ProviderEventFailureRow event={failure} key={failure.id} onReplay={onReplay} />
        ))}
        {failures.data?.length === 0 ? (
          <p className="text-muted-foreground rounded-md border p-3 text-sm">
            Aucun provider event en erreur.
          </p>
        ) : null}
        {failures.error ? (
          <p className="text-destructive rounded-md border p-3 text-sm">
            {failures.error instanceof Error
              ? failures.error.message
              : 'Provider events unavailable'}
          </p>
        ) : null}
      </div>
    </section>
  );
}

function ProviderEventFailureRow({
  event,
  onReplay,
}: {
  event: ProviderEventFailure;
  onReplay: (event: ProviderEventFailure) => void;
}) {
  const summary = formatPayloadSummary(event.payload_summary);

  return (
    <article className="bg-muted/40 rounded-md p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="mb-1 flex flex-wrap items-center gap-2">
            <h3 className="text-sm font-medium">
              {event.provider}:{event.event_type}
            </h3>
            <Badge variant="destructive">{event.status}</Badge>
            <Badge variant={event.signature_valid ? 'secondary' : 'destructive'}>
              signature {event.signature_valid ? 'ok' : 'invalid'}
            </Badge>
          </div>
          <p className="text-muted-foreground truncate font-mono text-xs">
            {event.provider_event_id}
          </p>
        </div>
        <ClipboardButton
          className="h-7 rounded-[min(var(--radius-md),12px)] text-[0.8rem]"
          label="Copy provider event ID"
          value={event.provider_event_id}
        />
        <Button onClick={() => onReplay(event)} size="sm" type="button">
          Replay form
        </Button>
      </div>
      <p className="text-muted-foreground mt-2 line-clamp-2 text-xs">{summary}</p>
      <p className="text-muted-foreground mt-2 text-xs">
        Received: {formatDate(event.received_at)}
      </p>
    </article>
  );
}

function formatPayloadSummary(value: unknown): string {
  if (value === null || value === undefined) return 'No payload summary.';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value);
  } catch {
    return 'Payload summary unavailable.';
  }
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
