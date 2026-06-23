import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Database, FileStack, Inbox, Users } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getCustomerCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  DormantWorkspace,
  HighStorageWorkspace,
  TenantPendingInvites,
} from './internal-admin.types';

export function CustomerCenterPanel({
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
  const customer = useQuery({
    queryKey: ['customer-center'],
    queryFn: () => getCustomerCenter(credentials),
    enabled: !disabled,
  });
  const data = customer.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="customer-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Customer center
          </p>
          <h2 className="text-base font-semibold">Adoption, usage et comptes a risque</h2>
        </div>
        <Badge variant={(data?.dormant_workspace_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.dormant_workspace_count)} dormant workspaces
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux customer." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-8">
        <CustomerMetric
          icon={Building2}
          label="Active tenants"
          value={formatCount(data?.active_tenant_count)}
        />
        <CustomerMetric
          icon={AlertTriangle}
          label="Suspended"
          tone={(data?.suspended_tenant_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.suspended_tenant_count)}
        />
        <CustomerMetric
          icon={AlertTriangle}
          label="Dormant"
          tone={(data?.dormant_workspace_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.dormant_workspace_count)}
        />
        <CustomerMetric
          icon={Inbox}
          label="Pending invites"
          value={formatCount(data?.pending_invitation_count)}
        />
        <CustomerMetric
          icon={Inbox}
          label="Expired invites"
          tone={(data?.expired_invitation_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.expired_invitation_count)}
        />
        <CustomerMetric
          icon={Users}
          label="Usage 24h"
          value={formatCount(data?.usage_events_24h)}
        />
        <CustomerMetric
          icon={Database}
          label="Storage"
          value={formatBytes(data?.storage_bytes_used)}
        />
        <CustomerMetric icon={FileStack} label="Files" value={formatCount(data?.file_count)} />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-3">
        <HighStorageList
          rows={data?.high_storage_workspaces ?? []}
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
        />
        <DormantWorkspaceList
          rows={data?.dormant_workspaces ?? []}
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
        />
        <PendingInviteList
          rows={data?.tenants_with_pending_invites ?? []}
          onSelectTenant={onSelectTenant}
        />
      </div>
    </section>
  );
}

function HighStorageList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: HighStorageWorkspace[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Top storage</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun quota usage." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.workspace_id}>
            <WorkspaceRowHeader
              badge={formatBytes(row.used_storage_bytes)}
              subtitle={`${row.tenant_name} - ${formatCount(row.file_count)} files`}
              title={row.workspace_name}
            />
            <EntityActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onWorkspace={() => onSelectWorkspace(row.workspace_id)}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function DormantWorkspaceList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: DormantWorkspace[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Workspaces dormants</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun workspace dormant." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.workspace_id}>
            <WorkspaceRowHeader
              badge={row.plan_code}
              subtitle={`${row.tenant_name} - last usage ${formatDate(row.last_usage_at)}`}
              title={row.workspace_name}
            />
            <EntityActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onWorkspace={() => onSelectWorkspace(row.workspace_id)}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function PendingInviteList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: TenantPendingInvites[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Invitations en attente</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun tenant bloque par invitations." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.tenant_id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.tenant_name}</p>
                <p className="text-muted-foreground text-xs">
                  oldest {formatDate(row.oldest_invitation_at)}
                </p>
              </div>
              <Badge variant="outline">{formatCount(row.pending_invitation_count)}</Badge>
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
              Open tenant
            </Button>
          </article>
        ))}
      </div>
    </div>
  );
}

function WorkspaceRowHeader({
  badge,
  subtitle,
  title,
}: {
  badge: string;
  subtitle: string;
  title: string;
}) {
  return (
    <div className="mb-2 flex items-center justify-between gap-2">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{title}</p>
        <p className="text-muted-foreground text-xs">{subtitle}</p>
      </div>
      <Badge variant="outline">{badge}</Badge>
    </div>
  );
}

function EntityActions({
  onTenant,
  onWorkspace,
}: {
  onTenant: () => void;
  onWorkspace: () => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button
        onClick={() => {
          onWorkspace();
          window.location.hash = 'workspace-detail';
        }}
        size="sm"
        type="button"
        variant="outline"
      >
        Workspace
      </Button>
      <Button
        onClick={() => {
          onTenant();
          window.location.hash = 'tenant-detail';
        }}
        size="sm"
        type="button"
        variant="ghost"
      >
        Tenant
      </Button>
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function CustomerMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default';
  value: string;
}) {
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={tone === 'danger' ? 'text-destructive size-4' : 'size-4'} />
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

function formatBytes(value: number | undefined): string {
  if (typeof value !== 'number') return '-';
  return new Intl.NumberFormat('fr-FR', {
    maximumFractionDigits: 1,
    style: 'unit',
    unit: value >= 1_000_000_000 ? 'gigabyte' : 'megabyte',
  }).format(value / (value >= 1_000_000_000 ? 1_000_000_000 : 1_000_000));
}

function formatDate(value: string | null): string {
  if (!value) return '-';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('fr-FR', {
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    month: '2-digit',
  });
}
