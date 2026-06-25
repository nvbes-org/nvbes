import { ExternalLink } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { AuditEvent } from './internal-admin.types';

type AuditTargetHandlers = {
  onSelectTenant?: (tenantId: string) => void;
  onSelectUser?: (principalId: string) => void;
  onSelectWorkspace?: (workspaceId: string) => void;
};

type AuditChange = {
  after: unknown;
  before: unknown;
  field: string;
};

export function AuditEventRow({
  event,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
}: {
  event: AuditEvent;
  onSelectTenant?: (tenantId: string) => void;
  onSelectUser?: (principalId: string) => void;
  onSelectWorkspace?: (workspaceId: string) => void;
}) {
  const metadata = summarizeMetadata(event.metadata);
  const changes = event.changes.length > 0 ? event.changes : auditChanges(event.metadata);
  const hashVariant = event.hash_chain_status === 'hash_anomaly' ? 'destructive' : 'secondary';

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
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={hashVariant}>{event.hash_chain_status}</Badge>
          <Badge variant="secondary">{formatDate(event.created_at)}</Badge>
        </div>
      </div>
      <div className="text-muted-foreground mt-2 grid gap-1 text-xs md:grid-cols-2">
        <span className="truncate">
          Actor: {event.actor_email ?? event.actor_principal_id ?? 'system'}
        </span>
        <span className="truncate">Event: {event.id}</span>
        <span className="truncate">Tenant: {event.tenant_id}</span>
        <span className="truncate">Workspace: {event.workspace_id ?? 'tenant-scope'}</span>
        <span className="truncate font-mono">Hash: {shortHash(event.event_hash)}</span>
        <span className="truncate font-mono">Previous: {shortHash(event.previous_event_hash)}</span>
      </div>
      {event.target_link ? (
        <div className="mt-3 flex flex-wrap gap-2">
          <Button
            onClick={() =>
              openTargetLink(event.target_link, {
                onSelectTenant,
                onSelectUser,
                onSelectWorkspace,
              })
            }
            size="sm"
            type="button"
            variant="outline"
          >
            <ExternalLink className="size-4" />
            Open {event.target_link.kind}
          </Button>
          <span className="text-muted-foreground self-center truncate font-mono text-xs">
            {event.target_link.label}
          </span>
        </div>
      ) : null}
      {changes.length > 0 ? (
        <div className="mt-3 rounded-md border bg-background">
          <div className="border-b px-2 py-1 text-xs font-medium">Diff avant/apres</div>
          <div className="divide-y">
            {changes.map((change) => (
              <div
                className="grid gap-1 px-2 py-2 text-xs md:grid-cols-[160px_1fr_1fr]"
                key={change.field}
              >
                <span className="font-medium">{change.field}</span>
                <span className="text-muted-foreground truncate">
                  Avant: {formatAuditValue(change.before)}
                </span>
                <span className="text-muted-foreground truncate">
                  Apres: {formatAuditValue(change.after)}
                </span>
              </div>
            ))}
          </div>
        </div>
      ) : null}
      {metadata ? (
        <pre className="bg-background text-muted-foreground mt-3 max-h-28 overflow-auto rounded-md border p-2 text-xs">
          {metadata}
        </pre>
      ) : null}
    </article>
  );
}

function openTargetLink(targetLink: AuditEvent['target_link'], handlers: AuditTargetHandlers) {
  if (!targetLink) return;
  if (targetLink.kind === 'tenant') handlers.onSelectTenant?.(targetLink.id);
  if (targetLink.kind === 'user') handlers.onSelectUser?.(targetLink.id);
  if (targetLink.kind === 'workspace') handlers.onSelectWorkspace?.(targetLink.id);
  window.location.hash = targetLink.href;
}

function auditChanges(metadata: unknown): AuditChange[] {
  if (!isRecord(metadata) || !Array.isArray(metadata.changes)) return [];
  return metadata.changes.flatMap((item) => {
    if (!isRecord(item) || typeof item.field !== 'string') return [];
    return [{ after: item.after, before: item.before, field: item.field }];
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function formatAuditValue(value: unknown): string {
  if (value === null || value === undefined) return 'null';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return value.toString();
  }
  return JSON.stringify(value);
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

function shortHash(value: string | null): string {
  if (!value) return 'missing';
  return value.length > 12 ? value.slice(0, 12) : value;
}
