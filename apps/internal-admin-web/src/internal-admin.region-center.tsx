import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Globe2, MapPinned, ShieldCheck } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getRegionCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  JurisdictionDistribution,
  MultiRegionTenant,
  RegionDistribution,
  RegionWorkspace,
} from './internal-admin.types';

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

function RegionDistributionList({ rows }: { rows: RegionDistribution[] }) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Region distribution</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucune region chargee." /> : null}
        {rows.map((row) => (
          <CompactRow
            key={row.data_region}
            label={row.data_region.toUpperCase()}
            meta={`${formatCount(row.tenant_count)} tenants`}
            value={`${formatCount(row.workspace_count)} workspaces`}
          />
        ))}
      </div>
    </div>
  );
}

function JurisdictionDistributionList({ rows }: { rows: JurisdictionDistribution[] }) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Jurisdiction distribution</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucune jurisdiction chargee." /> : null}
        {rows.map((row) => (
          <CompactRow
            key={row.jurisdiction}
            label={row.jurisdiction.toUpperCase()}
            meta={`${formatCount(row.tenant_count)} tenants`}
            value={`${formatCount(row.workspace_count)} workspaces`}
          />
        ))}
      </div>
    </div>
  );
}

function NonEuWorkspacesList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: RegionWorkspace[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Non-EU workspaces</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun workspace non-EU detecte." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.workspace_id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.workspace_name}</p>
                <p className="text-muted-foreground text-xs">
                  {row.tenant_name} - {row.jurisdiction.toUpperCase()}
                </p>
              </div>
              <Badge variant="destructive">{row.data_region.toUpperCase()}</Badge>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                onClick={() => {
                  onSelectWorkspace(row.workspace_id);
                  window.location.hash = 'workspace-detail';
                }}
                size="sm"
                type="button"
                variant="outline"
              >
                <Building2 className="size-4" />
                Open workspace
              </Button>
              <Button
                onClick={() => {
                  onSelectTenant(row.tenant_id);
                  window.location.hash = 'tenant-detail';
                }}
                size="sm"
                type="button"
                variant="ghost"
              >
                Tenant
              </Button>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}

function MultiRegionTenantsList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: MultiRegionTenant[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Multi-region tenants</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun tenant multi-region detecte." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.tenant_id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.tenant_name}</p>
                <p className="text-muted-foreground text-xs">
                  {formatCount(row.workspace_count)} workspaces across{' '}
                  {formatCount(row.region_count)} regions
                </p>
              </div>
              <Badge variant="outline">
                {row.regions.map((item) => item.toUpperCase()).join(', ')}
              </Badge>
            </div>
            <Button
              onClick={() => {
                onSelectTenant(row.tenant_id);
                window.location.hash = 'tenant-detail';
              }}
              size="sm"
              type="button"
              variant="outline"
            >
              <Building2 className="size-4" />
              Open tenant
            </Button>
          </article>
        ))}
      </div>
    </div>
  );
}

function CompactRow({ label, meta, value }: { label: string; meta: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 p-3">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{label}</p>
        <p className="text-muted-foreground text-xs">{meta}</p>
      </div>
      <Badge variant="secondary">{value}</Badge>
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
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
