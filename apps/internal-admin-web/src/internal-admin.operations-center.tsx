import { useQuery } from '@tanstack/react-query';
import {
  Activity,
  AlertTriangle,
  CalendarClock,
  FileSearch,
  FileDown,
  Mail,
  RefreshCcw,
  Shield,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getOperationsCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import { OperationsActionsPanel } from './internal-admin.operations-actions';
import type {
  AdminCredentials,
  RecentExportRun,
  RecentProviderFailure,
  RecentReconciliationDifference,
} from './internal-admin.types';

export function OperationsCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
}) {
  const operations = useQuery({
    queryKey: ['operations-center'],
    queryFn: () => getOperationsCenter(credentials),
    enabled: !disabled,
  });
  const data = operations.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="operations-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Operations center
          </p>
          <h2 className="text-base font-semibold">Backlog, jobs et reconciliation</h2>
        </div>
        <Badge
          variant={(data?.provider_event_failure_count ?? 0) > 0 ? 'destructive' : 'secondary'}
        >
          {formatCount(data?.provider_event_failure_count)} provider failures
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux operations." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <OpsMetric
          icon={AlertTriangle}
          label="Provider failures"
          tone={(data?.provider_event_failure_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.provider_event_failure_count)}
        />
        <OpsMetric
          icon={Activity}
          label="Provider backlog"
          tone={(data?.provider_event_backlog_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.provider_event_backlog_count)}
        />
        <OpsMetric
          icon={FileDown}
          label="Exports pending"
          value={formatCount(data?.export_pending_count)}
        />
        <OpsMetric
          icon={RefreshCcw}
          label="Recon pending"
          value={formatCount(data?.reconciliation_pending_count)}
        />
        <OpsMetric icon={Shield} label="Audit 24h" value={formatCount(data?.audit_events_24h)} />
        <OpsMetric
          icon={FileDown}
          label="Exports failed"
          tone={(data?.export_failed_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.export_failed_count)}
        />
        <OpsMetric
          icon={RefreshCcw}
          label="Recon failed"
          tone={(data?.reconciliation_failed_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.reconciliation_failed_count)}
        />
        <OpsMetric
          icon={AlertTriangle}
          label="Recon diffs"
          tone={(data?.unresolved_reconciliation_difference_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.unresolved_reconciliation_difference_count)}
        />
        <OpsMetric
          icon={AlertTriangle}
          label="Open incidents"
          tone={(data?.open_incident_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.open_incident_count)}
        />
        <OpsMetric
          icon={CalendarClock}
          label="Maintenance"
          value={formatCount(data?.scheduled_maintenance_window_count)}
        />
        <OpsMetric
          icon={Activity}
          label="Failed jobs"
          tone={(data?.failed_job_run_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.failed_job_run_count)}
        />
        <OpsMetric icon={Mail} label="Queued email" value={formatCount(data?.queued_email_count)} />
        <OpsMetric
          icon={Mail}
          label="Dropped email 24h"
          tone={(data?.dropped_email_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.dropped_email_count_24h)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-3">
        <ProviderFailuresList
          rows={data?.recent_provider_failures ?? []}
          onSelectTenant={onSelectTenant}
        />
        <ExportRunsList rows={data?.recent_export_runs ?? []} />
        <ReconciliationDiffList
          rows={data?.recent_reconciliation_differences ?? []}
          onSelectTenant={onSelectTenant}
        />
      </div>
      <OperationsActionsPanel credentials={credentials} disabled={disabled} />
    </section>
  );
}

function ProviderFailuresList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: RecentProviderFailure[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Provider failures</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucun provider event en erreur. Surveille le backlog et la signature provider." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <RowHeader
              badge={row.status}
              subtitle={`${row.provider} - ${formatDate(row.received_at)} - ${shortId(row.id)}`}
              title={row.event_type}
              tone="danger"
            />
            <TenantButton
              disabled={!row.tenant_id}
              onSelectTenant={onSelectTenant}
              tenantId={row.tenant_id}
            />
            <AuditButton targetId={row.id} targetType="billing_provider_event" />
          </article>
        ))}
      </div>
    </div>
  );
}

function ExportRunsList({ rows }: { rows: RecentExportRun[] }) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Export runs</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucun export run recent. Les echecs et relances apparaitront ici." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <RowHeader
              badge={row.status}
              subtitle={`${formatPeriod(row.period_start, row.period_end)} - ${formatDate(row.created_at)} - ${shortId(row.id)}`}
              title={row.export_type}
              tone={row.status === 'failed' || row.status === 'error' ? 'danger' : 'default'}
            />
            <AuditButton targetId={row.id} targetType="billing_export_run" />
          </article>
        ))}
      </div>
    </div>
  );
}

function ReconciliationDiffList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: RecentReconciliationDifference[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Recon differences</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucun ecart de reconciliation ouvert. Les differences provider/ledger seront listees ici." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <RowHeader
              badge={row.severity}
              subtitle={`${row.tenant_name ?? 'unmatched'} - ${formatDate(row.created_at)} - ${shortId(row.id)}`}
              title={row.difference_type}
              tone={row.severity === 'error' || row.severity === 'critical' ? 'danger' : 'default'}
            />
            <TenantButton
              disabled={!row.tenant_id}
              onSelectTenant={onSelectTenant}
              tenantId={row.tenant_id}
            />
            <AuditButton targetId={row.id} targetType="billing_reconciliation_difference" />
          </article>
        ))}
      </div>
    </div>
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

function TenantButton({
  disabled,
  onSelectTenant,
  tenantId,
}: {
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  tenantId: string | null;
}) {
  return (
    <Button
      disabled={disabled || !tenantId}
      onClick={() => {
        if (tenantId) {
          onSelectTenant(tenantId);
          window.location.hash = 'tenant-detail';
        }
      }}
      size="sm"
      type="button"
      variant="outline"
    >
      Open tenant
    </Button>
  );
}

function AuditButton({ targetId, targetType }: { targetId: string; targetType: string }) {
  return (
    <Button
      onClick={() => {
        window.location.hash = `audit?target_type=${targetType}&q=${targetId}`;
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

function OpsMetric({
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

function formatPeriod(start: string | null, end: string | null): string {
  if (!start && !end) return 'no period';
  return `${start ?? '-'} -> ${end ?? '-'}`;
}

function shortId(value: string): string {
  return value.slice(0, 8);
}
