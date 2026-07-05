import { useQuery } from '@tanstack/react-query';
import {
  AlertTriangle,
  ArrowRight,
  Building2,
  CheckCircle2,
  Clock3,
  ListChecks,
  ShieldCheck,
  Siren,
  Users,
  WalletCards,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { getCommandCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  CommandCenterSnapshot,
  CommandCenterWorkItem,
} from './internal-admin.types';

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
  const todayWork = buildTodayWork(data);

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
      <div className="mt-3 grid gap-4 xl:grid-cols-[1.35fr_1fr]">
        <div className="space-y-3">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <CommandMetric
              icon={Building2}
              label="Tenants"
              value={formatCount(data?.tenant_count)}
            />
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
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <CommandMetric
              icon={ListChecks}
              label="Approvals"
              tone={(data?.pending_approval_count ?? 0) > 0 ? 'danger' : 'default'}
              value={formatCount(data?.pending_approval_count)}
            />
            <CommandMetric
              icon={Siren}
              label="Incidents"
              tone={(data?.open_incident_count ?? 0) > 0 ? 'danger' : 'default'}
              value={formatCount(data?.open_incident_count)}
            />
            <CommandMetric
              icon={AlertTriangle}
              label="Audit anomalies"
              tone={(data?.audit_hash_anomaly_count ?? 0) > 0 ? 'danger' : 'default'}
              value={formatCount(data?.audit_hash_anomaly_count)}
            />
            <CommandMetric
              icon={WalletCards}
              label="SLA pressure"
              tone={(data?.sla_breach_count ?? 0) > 0 ? 'danger' : 'default'}
              value={formatCount(data?.sla_breach_count)}
            />
          </div>
        </div>
        <div className="bg-muted/20 rounded-md border p-3">
          <div className="mb-3 flex items-center justify-between gap-2">
            <div>
              <h3 className="text-sm font-semibold">Today&apos;s work</h3>
              <p className="text-muted-foreground text-xs">
                {todayWork.length > 0 ? `${todayWork.length} files actives` : 'Aucune file active'}
              </p>
            </div>
            {todayWork.length > 0 ? (
              <span className="text-muted-foreground text-xs">
                {formatCount(todayWork.reduce((sum, item) => sum + item.count, 0))} items
              </span>
            ) : null}
          </div>
          {todayWork.length === 0 ? (
            <CommandEmptyWork />
          ) : (
            <div className="space-y-2">
              {todayWork.map((item) => (
                <CommandWorkLink item={item} key={item.id} />
              ))}
            </div>
          )}
        </div>
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

function CommandWorkLink({ item }: { item: CommandCenterWorkItem }) {
  const isCritical = item.severity === 'critical';
  return (
    <a
      className="border-border bg-background hover:bg-muted/40 flex min-h-14 items-center justify-between gap-3 rounded-md border px-3 py-2 transition-colors"
      href={item.href}
    >
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span
            className={
              isCritical ? 'bg-destructive size-2 rounded-full' : 'bg-primary size-2 rounded-full'
            }
          />
          <span className="truncate text-sm font-medium">{item.label}</span>
        </div>
        <div className="text-muted-foreground mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs">
          <span>{item.owner}</span>
          <span>{item.severity}</span>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <span
          className={
            isCritical ? 'text-destructive text-sm font-semibold' : 'text-sm font-semibold'
          }
        >
          {formatCount(item.count)}
        </span>
        <ArrowRight className="text-muted-foreground size-4" />
      </div>
    </a>
  );
}

function CommandEmptyWork() {
  return (
    <div className="border-border bg-background flex min-h-32 items-center gap-3 rounded-md border p-3">
      <div className="bg-primary/10 text-primary flex size-9 shrink-0 items-center justify-center rounded-md">
        <CheckCircle2 className="size-4" />
      </div>
      <div>
        <div className="text-sm font-medium">Aucune action prioritaire</div>
        <div className="text-muted-foreground text-xs">
          Approvals, incidents, anomalies audit et SLA sont au nominal.
        </div>
      </div>
    </div>
  );
}

function buildTodayWork(data: CommandCenterSnapshot | undefined): CommandCenterWorkItem[] {
  if (!data) return [];
  if (data.today_work?.length) return data.today_work;
  return [
    workItem(
      'pending-approvals',
      'Pending approvals',
      data.pending_approval_count,
      'critical',
      '#pending-approvals',
      'Ops lead',
    ),
    workItem(
      'open-incidents',
      'Open incidents',
      data.open_incident_count,
      'critical',
      '#operations-center',
      'Operations',
    ),
    workItem(
      'audit-anomalies',
      'Audit anomalies',
      data.audit_hash_anomaly_count,
      'critical',
      '#audit-evidence-center',
      'Security',
    ),
    workItem(
      'sla-breaches',
      'SLA pressure',
      data.sla_breach_count,
      'high',
      '#revenue-center',
      'Revenue ops',
    ),
  ].filter((item): item is CommandCenterWorkItem => item !== null);
}

function workItem(
  id: string,
  label: string,
  count: number | undefined,
  severity: string,
  href: string,
  owner: string,
): CommandCenterWorkItem | null {
  if (!count || count <= 0) return null;
  return { count, href, id, label, owner, severity };
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
