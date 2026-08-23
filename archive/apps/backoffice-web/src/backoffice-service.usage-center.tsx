import { useQuery } from '@tanstack/react-query';
import {
  Activity,
  Building2,
  ChartNoAxesCombined,
  Gauge,
  PencilLine,
  RotateCcw,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getUsageCenter } from './backoffice-service.api';
import { LockedState } from './backoffice-service.locked-state';
import { UsageActionsPanel } from './backoffice-service.usage-actions';
import {
  CorrectionList,
  MeterUsageList,
  RollupList,
  TenantUsageList,
} from './backoffice-service.usage-lists';
import type { AdminCredentials } from './backoffice-service.types';

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
