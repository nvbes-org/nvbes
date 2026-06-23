import { ClipboardButton } from '@nvbes/web-ui';
import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Clock3, ShieldCheck, Users } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getTenantDetail } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function TenantDetailPanel({
  credentials,
  disabled,
  tenantId,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  tenantId: string | null;
}) {
  const tenant = useQuery({
    queryKey: ['tenant-detail', tenantId],
    queryFn: () => getTenantDetail(credentials, tenantId ?? ''),
    enabled: !disabled && tenantId !== null,
  });
  const data = tenant.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="tenant-detail">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Tenant detail
          </p>
          <h2 className="text-base font-semibold">
            {data ? data.name : tenantId ? 'Chargement tenant' : 'Aucun tenant selectionne'}
          </h2>
        </div>
        {data ? (
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary">{data.status}</Badge>
            <Badge variant="outline">{data.security_tier}</Badge>
          </div>
        ) : null}
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour ouvrir une fiche tenant." />
      ) : null}
      {!disabled && !tenantId ? (
        <div className="bg-muted/30 rounded-md border p-4 text-sm">
          Selectionne un tenant dans Global Search pour ouvrir sa fiche.
        </div>
      ) : null}
      {tenant.error ? (
        <div className="border-destructive/20 bg-destructive/5 text-destructive rounded-md border p-3 text-sm">
          {tenant.error instanceof Error ? tenant.error.message : 'Tenant unavailable'}
        </div>
      ) : null}
      {data ? (
        <div className="space-y-4">
          <div className="flex items-center justify-between gap-3 rounded-md border p-3">
            <div className="min-w-0">
              <p className="text-sm font-medium">{data.slug}</p>
              <p className="text-muted-foreground truncate font-mono text-xs">{data.id}</p>
            </div>
            <ClipboardButton
              className="h-7 rounded-[min(var(--radius-md),12px)] text-[0.8rem]"
              label="Copy tenant ID"
              value={data.id}
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
            <TenantMetric
              icon={Building2}
              label="Workspaces"
              value={formatCount(data.workspace_count)}
            />
            <TenantMetric icon={Users} label="Users" value={formatCount(data.user_count)} />
            <TenantMetric
              icon={Clock3}
              label="Audit 24h"
              value={formatCount(data.audit_events_24h)}
            />
            <TenantMetric
              icon={ShieldCheck}
              label="Open invoices"
              value={formatCount(data.open_invoice_count)}
            />
            <TenantMetric
              icon={AlertTriangle}
              label="Provider failures"
              tone={data.provider_failure_count > 0 ? 'danger' : 'default'}
              value={formatCount(data.provider_failure_count)}
            />
          </div>
        </div>
      ) : null}
    </section>
  );
}

function TenantMetric({
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
