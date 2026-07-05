import { FileSearch, Terminal } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type {
  DeveloperHealthIssue,
  ExpiringSecret,
  PendingMarketplaceApp,
  RiskyScope,
  WebhookFailure,
} from './internal-admin.types';

export function MarketplaceList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: PendingMarketplaceApp[];
}) {
  return (
    <DeveloperList
      emptyLabel="Aucune app marketplace en attente. Les soumissions a approuver seront listees ici."
      title="Marketplace review queue"
    >
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <LinkedTenantRow
            auditTargetId={row.id}
            auditTargetType="marketplace_app"
            badge={row.status}
            onSelectTenant={() => onSelectTenant(row.tenant_id)}
            subtitle={`${row.tenant_name} - ${row.client_id}`}
            title={row.client_name}
          />
        </article>
      ))}
    </DeveloperList>
  );
}

export function WebhookFailureList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: WebhookFailure[];
}) {
  return (
    <DeveloperList
      emptyLabel="Aucune livraison webhook en echec. Les retries, codes HTTP et erreurs provider apparaitront ici."
      title="Webhook failures"
    >
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <LinkedTenantRow
            auditTargetId={row.id}
            auditTargetType="webhook_delivery"
            badge={row.response_status ? String(row.response_status) : row.status}
            onSelectTenant={() => onSelectTenant(row.tenant_id)}
            subtitle={`${row.tenant_name} - ${row.event_type} - ${formatCount(row.attempt_count)} attempts`}
            title={row.endpoint_name}
          />
          {row.error_message ? (
            <p className="text-muted-foreground mt-2 line-clamp-2 text-xs">{row.error_message}</p>
          ) : null}
        </article>
      ))}
    </DeveloperList>
  );
}

export function ExpiringSecretList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: ExpiringSecret[];
}) {
  return (
    <DeveloperList
      emptyLabel="Aucun secret expire sous 14 jours. Les rotations a lancer seront listees ici."
      title="Secrets a renouveler"
    >
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <LinkedTenantRow
            auditTargetId={row.id}
            auditTargetType="developer_secret"
            badge={`last4 ${row.secret_last4}`}
            onSelectTenant={() => onSelectTenant(row.tenant_id)}
            subtitle={`${row.tenant_name} - expires ${formatDate(row.expires_at)}`}
            title={row.client_name}
          />
        </article>
      ))}
    </DeveloperList>
  );
}

export function RiskyScopeList({ rows }: { rows: RiskyScope[] }) {
  return (
    <DeveloperList
      emptyLabel="Aucun scope high/restricted actif. Les scopes sensibles et leurs proprietaires seront listes ici."
      title="Scopes sensibles"
    >
      {rows.map((row) => (
        <article className="p-3" key={row.scope_key}>
          <div className="mb-2 flex items-center justify-between gap-3">
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">{row.display_name}</p>
              <p className="text-muted-foreground text-xs">
                {row.scope_key} - {row.owner_team} - {row.lifecycle}
              </p>
            </div>
            <Badge variant={row.risk === 'restricted' ? 'destructive' : 'outline'}>
              {row.risk}
            </Badge>
          </div>
          <AuditButton targetId={row.scope_key} targetType="developer_scope" />
        </article>
      ))}
    </DeveloperList>
  );
}

export function HealthIssueList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: DeveloperHealthIssue[];
}) {
  return (
    <div className="xl:col-span-2">
      <DeveloperList
        emptyLabel="Aucun health check en warning ou failing. Les checks par cible seront visibles ici."
        title="Health checks"
      >
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <LinkedTenantRow
              auditTargetId={row.id}
              auditTargetType="developer_health_check"
              badge={row.status}
              onSelectTenant={() => onSelectTenant(row.tenant_id)}
              subtitle={`${row.tenant_name} - ${row.target_type}:${row.target_id}`}
              title={row.check_kind}
            />
            <p className="text-muted-foreground mt-2 line-clamp-2 text-xs">{row.summary}</p>
          </article>
        ))}
      </DeveloperList>
    </div>
  );
}

function LinkedTenantRow({
  auditTargetId,
  auditTargetType,
  badge,
  onSelectTenant,
  subtitle,
  title,
}: {
  auditTargetId: string;
  auditTargetType: string;
  badge: string;
  onSelectTenant: () => void;
  subtitle: string;
  title: string;
}) {
  return (
    <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{title}</p>
        <p className="text-muted-foreground text-xs">{subtitle}</p>
      </div>
      <div className="flex shrink-0 flex-wrap gap-2">
        <Badge variant="outline">{badge}</Badge>
        <Button
          onClick={() => {
            onSelectTenant();
            window.location.hash = 'tenant-detail';
          }}
          size="sm"
          type="button"
          variant="ghost"
        >
          <Terminal className="size-4" />
          Tenant
        </Button>
        <AuditButton targetId={auditTargetId} targetType={auditTargetType} />
      </div>
    </div>
  );
}

function DeveloperList({
  children,
  emptyLabel,
  title,
}: {
  children: ReactNode;
  emptyLabel: string;
  title: string;
}) {
  const rows = Array.isArray(children) ? children : [children];
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">{title}</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label={emptyLabel} /> : children}
      </div>
    </div>
  );
}

function AuditButton({ targetId, targetType }: { targetId: string; targetType: string }) {
  return (
    <Button
      onClick={() => {
        window.location.hash = `audit?target_type=${targetType}&q=${encodeURIComponent(targetId)}`;
      }}
      size="sm"
      type="button"
      variant="ghost"
    >
      <FileSearch className="size-4" />
      Audit
    </Button>
  );
}

function EmptyRow({ label }: { label: string }) {
  return (
    <div className="text-muted-foreground bg-muted/20 m-3 rounded-md border p-3 text-sm">
      {label}
    </div>
  );
}

function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
}

function formatDate(value: string | null): string {
  return value ? new Intl.DateTimeFormat('fr-FR').format(new Date(value)) : '-';
}
