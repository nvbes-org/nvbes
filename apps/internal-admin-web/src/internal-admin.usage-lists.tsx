import { Building2, FileSearch } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { MeterUsage, TenantUsage, UsageCorrection, UsageRollup } from './internal-admin.types';

export function MeterUsageList({ rows }: { rows: MeterUsage[] }) {
  return (
    <UsageList
      emptyLabel="Aucun event usage sur 24h. Les meters actifs et leur volume recent apparaitront ici."
      title="Meters 24h"
    >
      {rows.map((row) => (
        <CompactUsageRow
          auditTargetId={row.meter_code}
          auditTargetType="usage_meter"
          badge={row.unit}
          key={`${row.meter_code}:${row.unit}`}
          label={row.meter_code}
          meta={`${formatCount(row.quantity)} quantity - ${formatCount(row.event_count)} events`}
        />
      ))}
    </UsageList>
  );
}

export function TenantUsageList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: TenantUsage[];
}) {
  return (
    <UsageList
      emptyLabel="Aucun tenant actif sur 24h. Les plus gros consommateurs seront listes ici."
      title="Top tenants 24h"
    >
      {rows.map((row) => (
        <LinkedUsageRow
          auditTargetId={row.tenant_id}
          auditTargetType="tenant_usage"
          badge={`${formatCount(row.quantity)} qty`}
          key={row.tenant_id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${formatCount(row.event_count)} events`}
          title={row.tenant_name}
        />
      ))}
    </UsageList>
  );
}

export function RollupList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: UsageRollup[];
}) {
  return (
    <UsageList
      emptyLabel="Aucun rollup usage. Les agregats de periode courante seront visibles ici."
      title="Recent rollups"
    >
      {rows.map((row) => (
        <LinkedUsageRow
          auditTargetId={row.id}
          auditTargetType="usage_rollup"
          badge={`${formatCount(row.quantity)} ${row.unit}`}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectWorkspace={
            row.workspace_id ? () => onSelectWorkspace(row.workspace_id ?? '') : undefined
          }
          subtitle={`${row.tenant_name} - ${row.workspace_name ?? 'tenant scoped'} - ${formatDate(row.period_end)}`}
          title={row.meter_code}
        />
      ))}
    </UsageList>
  );
}

export function CorrectionList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: UsageCorrection[];
}) {
  return (
    <UsageList
      emptyLabel="Aucune correction usage recente. Les ajustements manuels et leurs raisons seront listes ici."
      title="Recent corrections"
    >
      {rows.map((row) => (
        <LinkedUsageRow
          auditTargetId={row.id}
          auditTargetType="usage_correction"
          badge={formatSigned(row.quantity_delta)}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.reason}`}
          title={row.meter_code}
          tone="warning"
        />
      ))}
    </UsageList>
  );
}

function CompactUsageRow({
  auditTargetId,
  auditTargetType,
  badge,
  label,
  meta,
}: {
  auditTargetId: string;
  auditTargetType: string;
  badge: string;
  label: string;
  meta: string;
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{label}</p>
          <p className="text-muted-foreground truncate text-xs">{meta}</p>
        </div>
        <Badge variant="secondary">{badge}</Badge>
      </div>
      <AuditButton targetId={auditTargetId} targetType={auditTargetType} />
    </article>
  );
}

function LinkedUsageRow({
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
  tone?: 'default' | 'warning';
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{title}</p>
          <p className="text-muted-foreground truncate text-xs">{subtitle}</p>
        </div>
        <Badge variant={tone === 'warning' ? 'outline' : 'secondary'}>{badge}</Badge>
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

function UsageList({
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

function formatSigned(value: number): string {
  return value > 0 ? `+${formatCount(value)}` : formatCount(value);
}
