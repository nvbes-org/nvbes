import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Clock3, ShieldCheck, Users, WalletCards } from 'lucide-react';
import type { ComponentType } from 'react';
import { getCommandCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function CommandCenterPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const commandCenter = useQuery({
    queryKey: ['command-center'],
    queryFn: () => getCommandCenter(credentials),
    enabled: !disabled,
  });
  const data = commandCenter.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="command-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Enterprise command center
          </p>
          <h2 className="text-base font-semibold">Pilotage global nvbes</h2>
        </div>
        <p className="text-muted-foreground text-xs">
          Dernier audit: {formatDate(data?.latest_audit_at)}
        </p>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux entreprise." />
      ) : null}
      <div className="mt-3 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <CommandMetric icon={Building2} label="Tenants" value={formatCount(data?.tenant_count)} />
        <CommandMetric
          icon={ShieldCheck}
          label="Workspaces"
          value={formatCount(data?.workspace_count)}
        />
        <CommandMetric icon={Users} label="Users" value={formatCount(data?.user_count)} />
        <CommandMetric
          icon={Clock3}
          label="Audit 24h"
          value={formatCount(data?.audit_events_24h)}
        />
        <CommandMetric
          icon={AlertTriangle}
          label="Provider failures"
          tone={(data?.billing_provider_failures ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.billing_provider_failures)}
        />
        <CommandMetric
          icon={AlertTriangle}
          label="Overdue invoices"
          tone={(data?.overdue_invoice_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.overdue_invoice_count)}
        />
        <CommandMetric
          icon={WalletCards}
          label="Failed payments"
          tone={(data?.failed_payment_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.failed_payment_count)}
        />
      </div>
    </section>
  );
}

function CommandMetric({
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

function formatDate(value: string | null | undefined): string {
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
