import { Building2, FileSearch } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type {
  JurisdictionDistribution,
  MultiRegionTenant,
  RegionDistribution,
  RegionWorkspace,
} from './backoffice-service.types';

export function RegionDistributionList({ rows }: { rows: RegionDistribution[] }) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Region distribution</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucune region chargee. Les volumes par residence seront visibles ici." />
        ) : null}
        {rows.map((row) => (
          <CompactRow
            key={row.data_region}
            label={row.data_region.toUpperCase()}
            meta={`${formatCount(row.tenant_count)} tenants`}
            targetId={row.data_region}
            targetType="data_region"
            value={`${formatCount(row.workspace_count)} workspaces`}
          />
        ))}
      </div>
    </div>
  );
}

export function JurisdictionDistributionList({ rows }: { rows: JurisdictionDistribution[] }) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Jurisdiction distribution</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucune jurisdiction chargee. Les obligations GDPR et non-GDPR seront consolidees ici." />
        ) : null}
        {rows.map((row) => (
          <CompactRow
            key={row.jurisdiction}
            label={row.jurisdiction.toUpperCase()}
            meta={`${formatCount(row.tenant_count)} tenants`}
            targetId={row.jurisdiction}
            targetType="jurisdiction"
            value={`${formatCount(row.workspace_count)} workspaces`}
          />
        ))}
      </div>
    </div>
  );
}

export function NonEuWorkspacesList({
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
        {rows.length === 0 ? (
          <EmptyRow label="Aucun workspace non-EU detecte. Les ecarts de residence et exceptions ouvertes apparaitront ici." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={row.workspace_id}>
            <RowHeader
              badge={row.data_region.toUpperCase()}
              subtitle={`${row.tenant_name} - ${row.jurisdiction.toUpperCase()}`}
              title={row.workspace_name}
              tone="danger"
            />
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
              <AuditButton targetId={row.workspace_id} targetType="region_workspace" />
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}

export function MultiRegionTenantsList({
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
        {rows.length === 0 ? (
          <EmptyRow label="Aucun tenant multi-region detecte. Les tenants repartis sur plusieurs residences seront listes ici." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={row.tenant_id}>
            <RowHeader
              badge={row.regions.map((item) => item.toUpperCase()).join(', ')}
              subtitle={`${formatCount(row.workspace_count)} workspaces across ${formatCount(row.region_count)} regions`}
              title={row.tenant_name}
            />
            <div className="flex flex-wrap gap-2">
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
              <AuditButton targetId={row.tenant_id} targetType="region_tenant" />
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}

function CompactRow({
  label,
  meta,
  targetId,
  targetType,
  value,
}: {
  label: string;
  meta: string;
  targetId: string;
  targetType: string;
  value: string;
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{label}</p>
          <p className="text-muted-foreground text-xs">{meta}</p>
        </div>
        <Badge variant="secondary">{value}</Badge>
      </div>
      <AuditButton targetId={targetId} targetType={targetType} />
    </article>
  );
}

function RowHeader({
  badge,
  subtitle,
  title,
  tone = 'default',
}: {
  badge: string;
  subtitle: string;
  title: string;
  tone?: 'danger' | 'default';
}) {
  return (
    <div className="mb-2 flex items-center justify-between gap-2">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{title}</p>
        <p className="text-muted-foreground text-xs">{subtitle}</p>
      </div>
      <Badge variant={tone === 'danger' ? 'destructive' : 'outline'}>{badge}</Badge>
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
