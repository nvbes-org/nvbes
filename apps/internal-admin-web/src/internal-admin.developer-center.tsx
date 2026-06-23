import { useQuery } from '@tanstack/react-query';
import {
  AlertTriangle,
  Code2,
  KeyRound,
  RadioTower,
  ShieldAlert,
  Store,
  Terminal,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getDeveloperCenter } from './internal-admin.api';
import { DeveloperActionsPanel } from './internal-admin.developer-actions';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  DeveloperHealthIssue,
  ExpiringSecret,
  PendingMarketplaceApp,
  RiskyScope,
  WebhookFailure,
} from './internal-admin.types';

export function DeveloperCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
}) {
  const developer = useQuery({
    queryKey: ['developer-center'],
    queryFn: () => getDeveloperCenter(credentials),
    enabled: !disabled,
  });
  const data = developer.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="developer-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Developer center
          </p>
          <h2 className="text-base font-semibold">Apps, webhooks, scopes et secrets</h2>
        </div>
        <Badge
          variant={(data?.failed_webhook_delivery_count_24h ?? 0) > 0 ? 'destructive' : 'secondary'}
        >
          {formatCount(data?.failed_webhook_delivery_count_24h)} webhook failures 24h
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux developer." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <DeveloperMetric
          icon={Code2}
          label="Active clients"
          value={formatCount(data?.active_client_count)}
        />
        <DeveloperMetric
          icon={Store}
          label="Marketplace pending"
          tone={(data?.pending_marketplace_app_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.pending_marketplace_app_count)}
        />
        <DeveloperMetric
          icon={RadioTower}
          label="Webhook failures"
          tone={(data?.failed_webhook_delivery_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.failed_webhook_delivery_count_24h)}
        />
        <DeveloperMetric
          icon={RadioTower}
          label="Active endpoints"
          value={formatCount(data?.active_webhook_endpoint_count)}
        />
        <DeveloperMetric
          icon={KeyRound}
          label="Secrets expiring"
          tone={(data?.expiring_secret_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.expiring_secret_count)}
        />
        <DeveloperMetric
          icon={ShieldAlert}
          label="Restricted scopes"
          tone={(data?.restricted_scope_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.restricted_scope_count)}
        />
        <DeveloperMetric
          icon={AlertTriangle}
          label="Health issues"
          tone={(data?.failing_health_check_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.failing_health_check_count)}
        />
      </div>
      <DeveloperActionsPanel credentials={credentials} disabled={disabled} />
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <MarketplaceList
          onSelectTenant={onSelectTenant}
          rows={data?.pending_marketplace_apps ?? []}
        />
        <WebhookFailureList onSelectTenant={onSelectTenant} rows={data?.webhook_failures ?? []} />
        <ExpiringSecretList onSelectTenant={onSelectTenant} rows={data?.expiring_secrets ?? []} />
        <RiskyScopeList rows={data?.risky_scopes ?? []} />
        <HealthIssueList onSelectTenant={onSelectTenant} rows={data?.health_issues ?? []} />
      </div>
    </section>
  );
}

function MarketplaceList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: PendingMarketplaceApp[];
}) {
  return (
    <DeveloperList title="Marketplace review queue" emptyLabel="Aucune app marketplace en attente.">
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <LinkedTenantRow
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

function WebhookFailureList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: WebhookFailure[];
}) {
  return (
    <DeveloperList title="Webhook failures" emptyLabel="Aucune livraison webhook en echec.">
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <LinkedTenantRow
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

function ExpiringSecretList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: ExpiringSecret[];
}) {
  return (
    <DeveloperList title="Secrets a renouveler" emptyLabel="Aucun secret expire sous 14 jours.">
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <LinkedTenantRow
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

function RiskyScopeList({ rows }: { rows: RiskyScope[] }) {
  return (
    <DeveloperList title="Scopes sensibles" emptyLabel="Aucun scope high/restricted actif.">
      {rows.map((row) => (
        <div className="flex items-center justify-between gap-3 p-3" key={row.scope_key}>
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{row.display_name}</p>
            <p className="text-muted-foreground text-xs">
              {row.scope_key} - {row.owner_team} - {row.lifecycle}
            </p>
          </div>
          <Badge variant={row.risk === 'restricted' ? 'destructive' : 'outline'}>{row.risk}</Badge>
        </div>
      ))}
    </DeveloperList>
  );
}

function HealthIssueList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: DeveloperHealthIssue[];
}) {
  return (
    <div className="xl:col-span-2">
      <DeveloperList title="Health checks" emptyLabel="Aucun health check en warning ou failing.">
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <LinkedTenantRow
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
  badge,
  onSelectTenant,
  subtitle,
  title,
}: {
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
      </div>
    </div>
  );
}

function DeveloperList({
  children,
  emptyLabel,
  title,
}: {
  children: React.ReactNode;
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

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function DeveloperMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default' | 'warning';
  value: string;
}) {
  const iconClass =
    tone === 'danger'
      ? 'text-destructive size-4'
      : tone === 'warning'
        ? 'text-amber-500 size-4'
        : 'size-4';

  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={iconClass} />
      </div>
      <div
        className={
          tone === 'danger' ? 'text-destructive text-xl font-semibold' : 'text-xl font-semibold'
        }
      >
        {value}
      </div>
    </div>
  );
}

function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
}

function formatDate(value: string | null): string {
  return value ? new Intl.DateTimeFormat('fr-FR').format(new Date(value)) : '-';
}
