import { useQuery } from '@tanstack/react-query';
import { BadgeCheck, Building2, Clock3, Copy, CreditCard, ShieldCheck, Users } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getWorkspaceDetail } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function WorkspaceDetailPanel({
  credentials,
  disabled,
  onSelectTenant,
  workspaceId,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  workspaceId: string | null;
}) {
  const workspace = useQuery({
    queryKey: ['workspace-detail', workspaceId],
    queryFn: () => getWorkspaceDetail(credentials, workspaceId ?? ''),
    enabled: !disabled && workspaceId !== null,
  });
  const data = workspace.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="workspace-detail">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Workspace detail
          </p>
          <h2 className="text-base font-semibold">
            {data
              ? data.name
              : workspaceId
                ? 'Chargement workspace'
                : 'Aucun workspace selectionne'}
          </h2>
        </div>
        {data ? (
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary">{data.workspace_type}</Badge>
            <Badge variant="outline">{data.plan_code}</Badge>
          </div>
        ) : null}
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour ouvrir une fiche workspace." />
      ) : null}
      {!disabled && !workspaceId ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          Selectionne un workspace dans Global Search pour ouvrir sa fiche.
        </div>
      ) : null}
      {workspace.error ? (
        <div className="border-destructive/20 bg-destructive/5 text-destructive rounded-md border p-3 text-sm">
          {workspace.error instanceof Error ? workspace.error.message : 'Workspace unavailable'}
        </div>
      ) : null}
      {data ? (
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-[1fr_auto]">
            <div className="min-w-0 rounded-md border p-3">
              <p className="text-muted-foreground text-xs">Tenant</p>
              <button
                className="hover:text-primary mt-1 text-left text-sm font-medium"
                onClick={() => {
                  onSelectTenant(data.tenant_id);
                  window.location.hash = 'tenant-detail';
                }}
                type="button"
              >
                {data.tenant_name}
              </button>
              <p className="text-muted-foreground mt-1 truncate font-mono text-xs">
                {data.tenant_id}
              </p>
            </div>
            <div className="flex items-center justify-between gap-3 rounded-md border p-3 md:min-w-80">
              <div className="min-w-0">
                <p className="text-sm font-medium">Workspace ID</p>
                <p className="text-muted-foreground truncate font-mono text-xs">{data.id}</p>
              </div>
              <Button
                onClick={() => void navigator.clipboard.writeText(data.id)}
                size="sm"
                type="button"
                variant="outline"
              >
                <Copy className="size-4" />
                Copy ID
              </Button>
            </div>
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
            <WorkspaceMetric icon={Users} label="Members" value={formatCount(data.member_count)} />
            <WorkspaceMetric
              icon={BadgeCheck}
              label="Owners"
              value={formatCount(data.owner_count)}
            />
            <WorkspaceMetric
              icon={ShieldCheck}
              label="Active"
              value={formatCount(data.active_member_count)}
            />
            <WorkspaceMetric
              icon={Building2}
              label="Service accounts"
              value={formatCount(data.service_account_count)}
            />
            <WorkspaceMetric
              icon={Clock3}
              label="Audit 24h"
              value={formatCount(data.audit_events_24h)}
            />
            <WorkspaceMetric
              icon={CreditCard}
              label="Active subs"
              value={formatCount(data.active_subscription_count)}
            />
            <WorkspaceMetric
              icon={CreditCard}
              label="Open invoices"
              tone={data.open_invoice_count > 0 ? 'danger' : 'default'}
              value={formatCount(data.open_invoice_count)}
            />
          </div>
        </div>
      ) : null}
    </section>
  );
}

function WorkspaceMetric({
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
