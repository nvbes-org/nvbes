import { Activity, AlertTriangle, ShieldAlert } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import type { AuditEvidenceAlert, RuntimeMetricAlert } from './internal-admin.types';

export function AuditEvidenceAlerts({
  alerts,
  runtimeAlerts,
}: {
  alerts: AuditEvidenceAlert[];
  runtimeAlerts: RuntimeMetricAlert[];
}) {
  if (alerts.length === 0 && runtimeAlerts.length === 0) {
    return null;
  }

  return (
    <div className="mt-4 grid gap-3 xl:grid-cols-2">
      <div className="rounded-md border">
        <div className="border-b p-3">
          <h3 className="text-sm font-medium">Evidence alerts</h3>
        </div>
        <div className="divide-y">
          {alerts.length === 0 ? (
            <EmptyAlert label="Aucune alerte evidence active." />
          ) : (
            alerts.map((alert) => <EvidenceAlertRow alert={alert} key={alert.id} />)
          )}
        </div>
      </div>
      <div className="rounded-md border">
        <div className="border-b p-3">
          <h3 className="text-sm font-medium">Runtime alert rules</h3>
        </div>
        <div className="divide-y">
          {runtimeAlerts.map((alert) => (
            <RuntimeAlertRow alert={alert} key={alert.id} />
          ))}
        </div>
      </div>
    </div>
  );
}

function EvidenceAlertRow({ alert }: { alert: AuditEvidenceAlert }) {
  return (
    <div className="flex items-start justify-between gap-3 p-3">
      <div className="min-w-0">
        <p className="flex items-center gap-2 text-sm font-medium">
          <ShieldAlert className={severityIconClass(alert.severity)} />
          {alert.title}
        </p>
        <p className="text-muted-foreground mt-1 truncate text-xs">{alert.target_anchor}</p>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <Badge variant={severityVariant(alert.severity)}>{alert.severity}</Badge>
        <Badge variant="outline">{formatCount(alert.count)}</Badge>
      </div>
    </div>
  );
}

function RuntimeAlertRow({ alert }: { alert: RuntimeMetricAlert }) {
  return (
    <div className="p-3">
      <div className="mb-2 flex items-start justify-between gap-3">
        <p className="flex min-w-0 items-center gap-2 text-sm font-medium">
          <AlertTriangle className={severityIconClass(alert.severity)} />
          <span className="truncate">{alert.title}</span>
        </p>
        <Badge variant={severityVariant(alert.severity)}>{alert.severity}</Badge>
      </div>
      <div className="text-muted-foreground flex items-center gap-2 font-mono text-[11px]">
        <Activity className="size-3" />
        <span className="truncate">{alert.metric_name}</span>
      </div>
      <p className="text-muted-foreground mt-1 truncate font-mono text-[11px]">{alert.condition}</p>
    </div>
  );
}

function EmptyAlert({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function severityVariant(severity: string): 'destructive' | 'outline' | 'secondary' {
  if (severity === 'critical') return 'destructive';
  if (severity === 'high') return 'secondary';
  return 'outline';
}

function severityIconClass(severity: string): string {
  return severity === 'critical' ? 'text-destructive size-4' : 'text-amber-500 size-4';
}

function formatCount(value: number): string {
  return new Intl.NumberFormat('fr-FR').format(value);
}
