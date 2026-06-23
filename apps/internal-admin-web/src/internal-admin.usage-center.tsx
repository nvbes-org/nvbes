import { useQuery } from '@tanstack/react-query';
import {
  Activity,
  Building2,
  ChartNoAxesCombined,
  Gauge,
  PencilLine,
  RotateCcw,
} from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getUsageCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import { UsageActionsPanel } from './internal-admin.usage-actions';
import type {
  AdminCredentials,
  MeterUsage,
  TenantUsage,
  UsageCorrection,
  UsageRollup,
} from './internal-admin.types';

export function UsageCenterPanel({
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
  const usage = useQuery({
    queryKey: ['usage-center'],
    queryFn: () => getUsageCenter(credentials),
    enabled: !disabled,
  });
  const data = usage.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="usage-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Usage center
          </p>
          <h2 className="text-base font-semibold">Meters, consommation et corrections</h2>
        </div>
        <Badge variant={(data?.correction_count_30d ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.correction_count_30d)} corrections 30d
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux usage." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        <UsageMetric
          icon={Gauge}
          label="Active meters"
          value={formatCount(data?.active_meter_count)}
        />
        <UsageMetric
          icon={Activity}
          label="Events 24h"
          value={formatCount(data?.usage_event_count_24h)}
        />
        <UsageMetric
          icon={ChartNoAxesCombined}
          label="Quantity 24h"
          value={formatCount(data?.usage_quantity_24h)}
        />
        <UsageMetric
          icon={PencilLine}
          label="Corrections 30d"
          tone={(data?.correction_count_30d ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.correction_count_30d)}
        />
        <UsageMetric
          icon={RotateCcw}
          label="Current rollups"
          value={formatCount(data?.rollup_count_current_period)}
        />
        <UsageMetric
          icon={Building2}
          label="Tenants 24h"
          value={formatCount(data?.distinct_tenant_count_24h)}
        />
      </div>
      <UsageActionsPanel credentials={credentials} disabled={disabled} />
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <MeterUsageList rows={data?.meter_usage_24h ?? []} />
        <TenantUsageList onSelectTenant={onSelectTenant} rows={data?.tenant_usage_24h ?? []} />
        <RollupList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.recent_rollups ?? []}
        />
        <CorrectionList onSelectTenant={onSelectTenant} rows={data?.recent_corrections ?? []} />
      </div>
    </section>
  );
}

function MeterUsageList({ rows }: { rows: MeterUsage[] }) {
  return (
    <UsageList emptyLabel="Aucun event usage sur 24h." title="Meters 24h">
      {rows.map((row) => (
        <CompactUsageRow
          badge={row.unit}
          key={`${row.meter_code}:${row.unit}`}
          label={row.meter_code}
          meta={`${formatCount(row.quantity)} quantity - ${formatCount(row.event_count)} events`}
        />
      ))}
    </UsageList>
  );
}

function TenantUsageList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: TenantUsage[];
}) {
  return (
    <UsageList emptyLabel="Aucun tenant actif sur 24h." title="Top tenants 24h">
      {rows.map((row) => (
        <LinkedUsageRow
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

function RollupList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: UsageRollup[];
}) {
  return (
    <UsageList emptyLabel="Aucun rollup usage." title="Recent rollups">
      {rows.map((row) => (
        <LinkedUsageRow
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

function CorrectionList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: UsageCorrection[];
}) {
  return (
    <UsageList emptyLabel="Aucune correction usage recente." title="Recent corrections">
      {rows.map((row) => (
        <LinkedUsageRow
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

function CompactUsageRow({ badge, label, meta }: { badge: string; label: string; meta: string }) {
  return (
    <div className="flex items-center justify-between gap-3 p-3">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{label}</p>
        <p className="text-muted-foreground truncate text-xs">{meta}</p>
      </div>
      <Badge variant="secondary">{badge}</Badge>
    </div>
  );
}

function LinkedUsageRow({
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

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function UsageMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'default' | 'warning';
  value: string;
}) {
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={tone === 'warning' ? 'text-amber-500 size-4' : 'size-4'} />
      </div>
      <div className="text-xl font-semibold">{value}</div>
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
