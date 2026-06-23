import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Gauge, PackageCheck, ScrollText, Sparkles } from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getEntitlementsCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  EntitlementPlan,
  ExpiringEntitlement,
  OverQuotaBalance,
  UnpublishedEntitlementChange,
} from './internal-admin.types';

export function EntitlementsCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectWorkspace,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
}) {
  const entitlements = useQuery({
    queryKey: ['entitlements-center'],
    queryFn: () => getEntitlementsCenter(credentials),
    enabled: !disabled,
  });
  const data = entitlements.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="entitlements-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Entitlements center
          </p>
          <h2 className="text-base font-semibold">Plans, features, quotas et droits produit</h2>
        </div>
        <Badge variant={(data?.over_quota_balance_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.over_quota_balance_count)} over quota
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux entitlements." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <EntitlementMetric
          icon={PackageCheck}
          label="Active plans"
          value={formatCount(data?.active_plan_count)}
        />
        <EntitlementMetric
          icon={Sparkles}
          label="Features"
          value={formatCount(data?.active_feature_count)}
        />
        <EntitlementMetric
          icon={Gauge}
          label="Quota defs"
          value={formatCount(data?.quota_definition_count)}
        />
        <EntitlementMetric
          icon={PackageCheck}
          label="Active rights"
          value={formatCount(data?.active_entitlement_count)}
        />
        <EntitlementMetric
          icon={AlertTriangle}
          label="Over quota"
          tone={(data?.over_quota_balance_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.over_quota_balance_count)}
        />
        <EntitlementMetric
          icon={ScrollText}
          label="Unpublished"
          tone={(data?.unpublished_change_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.unpublished_change_count)}
        />
        <EntitlementMetric
          icon={Sparkles}
          label="Trials active"
          value={formatCount(data?.active_trial_grant_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <PlanList rows={data?.active_plans ?? []} />
        <OverQuotaList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.over_quota_balances ?? []}
        />
        <ExpiringEntitlementList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.expiring_entitlements ?? []}
        />
        <UnpublishedChangeList
          onSelectTenant={onSelectTenant}
          rows={data?.unpublished_changes ?? []}
        />
      </div>
    </section>
  );
}

function PlanList({ rows }: { rows: EntitlementPlan[] }) {
  return (
    <EntitlementList emptyLabel="Aucun plan actif." title="Active catalog">
      {rows.map((row) => (
        <div className="flex items-center justify-between gap-3 p-3" key={row.plan_id}>
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{row.plan_name}</p>
            <p className="text-muted-foreground truncate text-xs">
              {row.product_name} - {row.plan_code} - {formatCount(row.active_version_count)}{' '}
              versions
            </p>
          </div>
          <Badge variant="secondary">{formatCount(row.feature_count)} features</Badge>
        </div>
      ))}
    </EntitlementList>
  );
}

function OverQuotaList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: OverQuotaBalance[];
}) {
  return (
    <EntitlementList emptyLabel="Aucun quota depasse." title="Over quota balances">
      {rows.map((row) => (
        <LinkedEntitlementRow
          badge={`${formatCount(row.used_quantity - row.included_quantity)} over`}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectWorkspace={
            row.workspace_id ? () => onSelectWorkspace(row.workspace_id ?? '') : undefined
          }
          subtitle={`${row.tenant_name} - used ${formatCount(row.used_quantity)} / ${formatCount(row.included_quantity)}`}
          title={row.quota_code}
          tone="danger"
        />
      ))}
    </EntitlementList>
  );
}

function ExpiringEntitlementList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: ExpiringEntitlement[];
}) {
  return (
    <EntitlementList emptyLabel="Aucun entitlement expire sous 14 jours." title="Expiring rights">
      {rows.map((row) => (
        <LinkedEntitlementRow
          badge={formatDate(row.effective_to)}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectWorkspace={
            row.workspace_id ? () => onSelectWorkspace(row.workspace_id ?? '') : undefined
          }
          subtitle={`${row.tenant_name} - ${row.workspace_name ?? 'tenant scoped'}`}
          title={row.status}
          tone="warning"
        />
      ))}
    </EntitlementList>
  );
}

function UnpublishedChangeList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: UnpublishedEntitlementChange[];
}) {
  return (
    <EntitlementList
      emptyLabel="Aucun changement entitlement non publie."
      title="Unpublished changes"
    >
      {rows.map((row) => (
        <LinkedEntitlementRow
          badge={formatDate(row.created_at)}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={row.tenant_name}
          title={row.event_id}
          tone="warning"
        />
      ))}
    </EntitlementList>
  );
}

function LinkedEntitlementRow({
  badge,
  onSelectTenant,
  onSelectWorkspace,
  subtitle,
  title,
  tone = 'default',
}: {
  badge: string;
  onSelectTenant: () => void;
  onSelectWorkspace?: () => void;
  subtitle: string;
  title: string;
  tone?: 'danger' | 'default' | 'warning';
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{title}</p>
          <p className="text-muted-foreground truncate text-xs">{subtitle}</p>
        </div>
        <Badge variant={tone === 'danger' ? 'destructive' : 'outline'}>{badge}</Badge>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          onClick={() => {
            onSelectTenant();
            window.location.hash = 'tenant-detail';
          }}
          size="sm"
          type="button"
          variant="ghost"
        >
          <Building2 className="size-4" />
          Tenant
        </Button>
        {onSelectWorkspace ? (
          <Button
            onClick={() => {
              onSelectWorkspace();
              window.location.hash = 'workspace-detail';
            }}
            size="sm"
            type="button"
            variant="outline"
          >
            Workspace
          </Button>
        ) : null}
      </div>
    </article>
  );
}

function EntitlementList({
  children,
  emptyLabel,
  title,
}: {
  children: ReactNode[];
  emptyLabel: string;
  title: string;
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">{title}</h3>
      </div>
      <div className="divide-y">
        {children.length === 0 ? <EmptyRow label={emptyLabel} /> : children}
      </div>
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function EntitlementMetric({
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

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}
