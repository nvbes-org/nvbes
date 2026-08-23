import { Building2, FileSearch } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type {
  EntitlementPlan,
  ExpiringEntitlement,
  OverQuotaBalance,
  UnpublishedEntitlementChange,
} from './backoffice-service.types';

export function PlanList({ rows }: { rows: EntitlementPlan[] }) {
  return (
    <EntitlementList
      emptyLabel="Aucun plan actif. Les plans publies, versions et features du catalogue apparaitront ici."
      title="Active catalog"
    >
      {rows.map((row) => (
        <article className="p-3" key={row.plan_id}>
          <div className="mb-2 flex items-center justify-between gap-3">
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">{row.plan_name}</p>
              <p className="text-muted-foreground truncate text-xs">
                {row.product_name} - {row.plan_code} - {formatCount(row.active_version_count)}{' '}
                versions
              </p>
            </div>
            <Badge variant="secondary">{formatCount(row.feature_count)} features</Badge>
          </div>
          <AuditButton targetId={row.plan_id} targetType="entitlement_plan" />
        </article>
      ))}
    </EntitlementList>
  );
}

export function OverQuotaList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: OverQuotaBalance[];
}) {
  return (
    <EntitlementList
      emptyLabel="Aucun quota depasse. Les consommations au-dela des droits inclus seront listees ici."
      title="Over quota balances"
    >
      {rows.map((row) => (
        <LinkedEntitlementRow
          auditTargetId={row.id}
          auditTargetType="over_quota_balance"
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

export function ExpiringEntitlementList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: ExpiringEntitlement[];
}) {
  return (
    <EntitlementList
      emptyLabel="Aucun entitlement expire sous 14 jours. Les droits a renouveler ou retirer apparaitront ici."
      title="Expiring rights"
    >
      {rows.map((row) => (
        <LinkedEntitlementRow
          auditTargetId={row.id}
          auditTargetType="expiring_entitlement"
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

export function UnpublishedChangeList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: UnpublishedEntitlementChange[];
}) {
  return (
    <EntitlementList
      emptyLabel="Aucun changement entitlement non publie. Les modifications en attente de publication seront listees ici."
      title="Unpublished changes"
    >
      {rows.map((row) => (
        <LinkedEntitlementRow
          auditTargetId={row.id}
          auditTargetType="unpublished_entitlement_change"
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
  auditTargetId,
  auditTargetType,
  badge,
  onSelectTenant,
  onSelectWorkspace,
  subtitle,
  title,
  tone = 'default',
}: {
  auditTargetId: string;
  auditTargetType: string;
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
        <AuditButton targetId={auditTargetId} targetType={auditTargetType} />
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

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}
