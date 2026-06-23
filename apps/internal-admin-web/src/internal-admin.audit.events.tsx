import { useQuery } from '@tanstack/react-query';
import { RefreshCw, Search, ShieldCheck } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { listAuditEvents } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, AuditEvent } from './internal-admin.types';

const allFilterValue = '__all__';
const targetTypeOptions = ['tenant', 'workspace', 'principal', 'runbook', 'billing_invoice'];
const actionOptions = [
  'internal_admin.tenant.suspend',
  'internal_admin.tenant.reactivate',
  'internal_admin.workspace.suspend',
  'internal_admin.workspace.reactivate',
  'internal_admin.user.suspend',
  'internal_admin.user.reactivate',
  'internal_admin.runbook.executed',
];

export function AuditEventsPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const [action, setAction] = useState(allFilterValue);
  const [targetType, setTargetType] = useState(allFilterValue);
  const [query, setQuery] = useState('');
  const filters = useMemo(
    () => ({
      action: action === allFilterValue ? undefined : action,
      limit: 50,
      query,
      targetType: targetType === allFilterValue ? undefined : targetType,
    }),
    [action, query, targetType],
  );
  const auditEvents = useQuery({
    queryKey: ['internal-audit-events', credentials.workspaceId, filters],
    queryFn: () => listAuditEvents(credentials, filters),
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

      <div className="mb-4 grid gap-2 lg:grid-cols-[1fr_auto_auto_auto]">
        <div className="relative">
          <Search className="text-muted-foreground pointer-events-none absolute top-2 left-2 size-4" />
          <Input
            className="pl-8"
            disabled={disabled}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search action, actor, target..."
            value={query}
          />
        </div>
        <Select disabled={disabled} onValueChange={setAction} value={action}>
          <SelectTrigger className="w-full lg:w-72">
            <SelectValue placeholder="Action" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={allFilterValue}>Toutes les actions</SelectItem>
            {actionOptions.map((item) => (
              <SelectItem key={item} value={item}>
                {item}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select disabled={disabled} onValueChange={setTargetType} value={targetType}>
          <SelectTrigger className="w-full lg:w-44">
            <SelectValue placeholder="Target" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={allFilterValue}>Toutes les cibles</SelectItem>
            {targetTypeOptions.map((item) => (
              <SelectItem key={item} value={item}>
                {item}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          disabled={disabled || auditEvents.isFetching}
          onClick={() => {
            setAction(allFilterValue);
            setTargetType(allFilterValue);
            setQuery('');
          }}
          type="button"
          variant="outline"
        >
          Reset
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
  const metadata = summarizeMetadata(event.metadata);

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
      {metadata ? (
        <pre className="bg-background text-muted-foreground mt-3 max-h-28 overflow-auto rounded-md border p-2 text-xs">
          {metadata}
        </pre>
      ) : null}
    </article>
  );
}

function summarizeMetadata(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return value.toString();
  }
  if (typeof value !== 'object') return null;
  const json = JSON.stringify(value, null, 2);
  return json === '{}' ? null : json;
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
