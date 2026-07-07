import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Globe2, MapPinned, ShieldCheck } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getRegionCenter } from './backoffice-service.api';
import { LockedState } from './backoffice-service.locked-state';
import { RegionActionsPanel } from './backoffice-service.region-actions';
import {
  JurisdictionDistributionList,
  MultiRegionTenantsList,
  NonEuWorkspacesList,
  RegionDistributionList,
} from './backoffice-service.region-lists';
import type { AdminCredentials } from './backoffice-service.types';

export function RegionCenterPanel({
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
  const region = useQuery({
    queryKey: ['region-center'],
    queryFn: () => getRegionCenter(credentials),
    enabled: !disabled,
  });
  const data = region.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="region-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Region center
          </p>
          <h2 className="text-base font-semibold">Data residency et juridictions</h2>
        </div>
        <Badge variant={(data?.non_eu_workspace_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.non_eu_workspace_count)} non-EU
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux region." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <RegionMetric
          icon={Globe2}
          label="EU workspaces"
          value={formatCount(data?.eu_workspace_count)}
        />
        <RegionMetric
          icon={MapPinned}
          label="Non-EU"
          tone={(data?.non_eu_workspace_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.non_eu_workspace_count)}
        />
        <RegionMetric
          icon={ShieldCheck}
          label="GDPR"
          value={formatCount(data?.gdpr_workspace_count)}
        />
        <RegionMetric
          icon={AlertTriangle}
          label="Non-GDPR"
          tone={(data?.non_gdpr_workspace_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.non_gdpr_workspace_count)}
        />
        <RegionMetric
          icon={Building2}
          label="Multi-region tenants"
          tone={(data?.multi_region_tenant_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.multi_region_tenant_count)}
        />
      </div>
      <RegionActionsPanel credentials={credentials} disabled={disabled} />
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <RegionDistributionList rows={data?.region_distribution ?? []} />
        <JurisdictionDistributionList rows={data?.jurisdiction_distribution ?? []} />
        <NonEuWorkspacesList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.non_eu_workspaces ?? []}
        />
        <MultiRegionTenantsList
          onSelectTenant={onSelectTenant}
          rows={data?.multi_region_tenants ?? []}
        />
      </div>
    </section>
  );
}

function RegionMetric({
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
