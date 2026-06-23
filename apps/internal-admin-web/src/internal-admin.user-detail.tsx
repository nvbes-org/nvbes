import { useQuery } from '@tanstack/react-query';
import { Clock3, Copy, KeyRound, ShieldAlert, ShieldCheck, UserRound, Users } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getUserDetail } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function UserDetailPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectWorkspace,
  principalId,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  principalId: string | null;
}) {
  const user = useQuery({
    queryKey: ['user-detail', principalId],
    queryFn: () => getUserDetail(credentials, principalId ?? ''),
    enabled: !disabled && principalId !== null,
  });
  const data = user.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="user-detail">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            User detail
          </p>
          <h2 className="text-base font-semibold">
            {data ? data.name : principalId ? 'Chargement user' : 'Aucun user selectionne'}
          </h2>
        </div>
        {data ? (
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary">{data.user_status}</Badge>
            <Badge variant="outline">{data.principal_status}</Badge>
          </div>
        ) : null}
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour ouvrir une fiche user." />
      ) : null}
      {!disabled && !principalId ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          Selectionne un user dans Global Search pour ouvrir sa fiche.
        </div>
      ) : null}
      {user.error ? (
        <div className="border-destructive/20 bg-destructive/5 text-destructive rounded-md border p-3 text-sm">
          {user.error instanceof Error ? user.error.message : 'User unavailable'}
        </div>
      ) : null}
      {data ? (
        <div className="space-y-4">
          <div className="grid gap-3 lg:grid-cols-[1.2fr_1fr_1fr]">
            <div className="min-w-0 rounded-md border p-3">
              <p className="text-sm font-medium">{data.email}</p>
              <p className="text-muted-foreground mt-1 truncate font-mono text-xs">
                {data.principal_id}
              </p>
              <Button
                className="mt-3"
                onClick={() => void navigator.clipboard.writeText(data.principal_id)}
                size="sm"
                type="button"
                variant="outline"
              >
                <Copy className="size-4" />
                Copy principal
              </Button>
            </div>
            <LinkCard
              id={data.tenant_id}
              label="Tenant"
              name={data.tenant_name}
              onOpen={() => {
                onSelectTenant(data.tenant_id);
                window.location.hash = 'tenant-detail';
              }}
            />
            <LinkCard
              id={data.primary_workspace_id}
              label="Primary workspace"
              name={data.primary_workspace_name}
              onOpen={() => {
                if (data.primary_workspace_id) {
                  onSelectWorkspace(data.primary_workspace_id);
                  window.location.hash = 'workspace-detail';
                }
              }}
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
            <UserMetric icon={Users} label="Workspaces" value={formatCount(data.workspace_count)} />
            <UserMetric
              icon={UserRound}
              label="Active access"
              value={formatCount(data.active_workspace_count)}
            />
            <UserMetric
              icon={ShieldCheck}
              label="MFA active"
              tone={data.active_mfa_factor_count === 0 ? 'danger' : 'default'}
              value={formatCount(data.active_mfa_factor_count)}
            />
            <UserMetric
              icon={KeyRound}
              label="OAuth consents"
              value={formatCount(data.active_oauth_consent_count)}
            />
            <UserMetric
              icon={ShieldAlert}
              label="Risk 24h"
              tone={data.risk_events_24h > 0 ? 'danger' : 'default'}
              value={formatCount(data.risk_events_24h)}
            />
            <UserMetric
              icon={Clock3}
              label="Audit 24h"
              value={formatCount(data.audit_events_24h)}
            />
          </div>
        </div>
      ) : null}
    </section>
  );
}

function LinkCard({
  id,
  label,
  name,
  onOpen,
}: {
  id: string | null;
  label: string;
  name: string | null;
  onOpen: () => void;
}) {
  return (
    <div className="min-w-0 rounded-md border p-3">
      <p className="text-muted-foreground text-xs">{label}</p>
      <button
        className="hover:text-primary mt-1 text-left text-sm font-medium disabled:pointer-events-none disabled:opacity-60"
        disabled={!id}
        onClick={onOpen}
        type="button"
      >
        {name ?? 'Aucun lien'}
      </button>
      <p className="text-muted-foreground mt-1 truncate font-mono text-xs">{id ?? '-'}</p>
    </div>
  );
}

function UserMetric({
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
          tone === 'danger' ? 'text-destructive text-lg font-semibold' : 'text-lg font-semibold'
        }
      >
        {value}
      </div>
    </div>
  );
}

function formatCount(value: number): string {
  return new Intl.NumberFormat('fr-FR').format(value);
}
