import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, KeyRound, ShieldCheck, Users } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getAccessCenter } from './backoffice-service.api';
import { PrivilegedMembershipAction } from './backoffice-service.access-actions';
import { LockedState } from './backoffice-service.locked-state';
import type {
  AdminCredentials,
  OwnerlessWorkspace,
  PrivilegedUser,
  StaleServiceAccount,
} from './backoffice-service.types';

export function AccessCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
}) {
  const access = useQuery({
    queryKey: ['access-center'],
    queryFn: () => getAccessCenter(credentials),
    enabled: !disabled,
  });
  const data = access.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="access-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Access center
          </p>
          <h2 className="text-base font-semibold">Privileges, owners et comptes machine</h2>
        </div>
        <Badge variant={(data?.ownerless_workspace_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.ownerless_workspace_count)} ownerless
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux access." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-8">
        <AccessMetric
          icon={ShieldCheck}
          label="Owners"
          value={formatCount(data?.workspace_owner_count)}
        />
        <AccessMetric
          icon={Users}
          label="Admins"
          value={formatCount(data?.workspace_admin_count)}
        />
        <AccessMetric
          icon={AlertTriangle}
          label="Ownerless"
          tone={(data?.ownerless_workspace_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.ownerless_workspace_count)}
        />
        <AccessMetric
          icon={KeyRound}
          label="Service accounts"
          value={formatCount(data?.service_account_count)}
        />
        <AccessMetric
          icon={AlertTriangle}
          label="Stale SA"
          tone={(data?.stale_service_account_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.stale_service_account_count)}
        />
        <AccessMetric
          icon={KeyRound}
          label="OAuth clients"
          value={formatCount(data?.oauth_client_count)}
        />
        <AccessMetric
          icon={KeyRound}
          label="Revoked clients"
          value={formatCount(data?.revoked_oauth_client_count)}
        />
        <AccessMetric
          icon={AlertTriangle}
          label="Restricted policies"
          tone={(data?.restricted_client_policy_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.restricted_client_policy_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-3">
        <PrivilegedUsersList
          credentials={credentials}
          disabled={disabled}
          rows={data?.privileged_users ?? []}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          onSelectWorkspace={onSelectWorkspace}
        />
        <OwnerlessWorkspaceList
          rows={data?.ownerless_workspaces ?? []}
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
        />
        <StaleServiceAccountList
          rows={data?.stale_service_accounts ?? []}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          onSelectWorkspace={onSelectWorkspace}
        />
      </div>
    </section>
  );
}

function PrivilegedUsersList({
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
  rows,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: PrivilegedUser[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Privileged users</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun owner/admin actif." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={`${row.workspace_id}:${row.principal_id}`}>
            <RowHeader
              badge={row.role}
              subtitle={`${row.workspace_name} - ${row.tenant_name}`}
              title={row.email ?? row.name ?? row.principal_id}
            />
            <EntityActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onUser={() => onSelectUser(row.principal_id)}
              onWorkspace={() => onSelectWorkspace(row.workspace_id)}
            />
            <PrivilegedMembershipAction credentials={credentials} disabled={disabled} row={row} />
          </article>
        ))}
      </div>
    </div>
  );
}

function OwnerlessWorkspaceList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: OwnerlessWorkspace[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Workspaces sans owner</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Tous les workspaces ont un owner actif." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.workspace_id}>
            <RowHeader
              badge={row.plan_code}
              subtitle={`${row.tenant_name} - created ${formatDate(row.created_at)}`}
              title={row.workspace_name}
            />
            <WorkspaceTenantActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onWorkspace={() => onSelectWorkspace(row.workspace_id)}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function StaleServiceAccountList({
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: StaleServiceAccount[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Service accounts stale</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun service account stale." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.principal_id}>
            <RowHeader
              badge={row.last_rotated_at ? formatDate(row.last_rotated_at) : 'never rotated'}
              subtitle={`${row.workspace_name ?? 'tenant scoped'} - ${row.tenant_name}`}
              title={row.name}
            />
            <EntityActions
              disableWorkspace={!row.workspace_id}
              onTenant={() => onSelectTenant(row.tenant_id)}
              onUser={() => onSelectUser(row.principal_id)}
              onWorkspace={() => {
                if (row.workspace_id) onSelectWorkspace(row.workspace_id);
              }}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function RowHeader({ badge, subtitle, title }: { badge: string; subtitle: string; title: string }) {
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
  disableWorkspace = false,
  onTenant,
  onUser,
  onWorkspace,
}: {
  disableWorkspace?: boolean;
  onTenant: () => void;
  onUser: () => void;
  onWorkspace: () => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button
        onClick={() => {
          onUser();
          window.location.hash = 'user-detail';
        }}
        size="sm"
        type="button"
        variant="outline"
      >
        User
      </Button>
      <Button
        disabled={disableWorkspace}
        onClick={() => {
          onWorkspace();
          window.location.hash = 'workspace-detail';
        }}
        size="sm"
        type="button"
        variant="ghost"
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

function WorkspaceTenantActions({
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

function AccessMetric({
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

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('fr-FR', {
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    month: '2-digit',
  });
}
