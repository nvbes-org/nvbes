import { Activity, Building2, ShieldCheck, Users } from 'lucide-react';
import { EnterpriseRouteState } from '../components/EnterpriseRouteState';

const overviewMetrics = [
  { label: 'Active users', value: 'Pending', icon: Users },
  { label: 'Workspaces', value: 'Pending', icon: Building2 },
  { label: 'Policy coverage', value: 'Pending', icon: ShieldCheck },
  { label: 'Usage signals', value: 'Pending', icon: Activity },
] as const;

const overviewModules = [
  'Users and access lifecycle',
  'Workspace governance',
  'Security and audit posture',
  'Billing and usage controls',
] as const;

export function OverviewPage() {
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-2">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          Tenant command center
        </p>
        <div className="flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
          <div>
            <h1 className="text-2xl font-heading font-semibold tracking-normal">Overview</h1>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
              A dense operational view for tenant administration across users, workspaces,
              governance, billing, and platform usage.
            </p>
          </div>
          <div className="h-9 shrink-0 rounded-md border border-border px-3 py-2 text-xs font-medium text-muted-foreground">
            Shell only
          </div>
        </div>
      </header>

      <section className="grid gap-3 md:grid-cols-4">
        {overviewMetrics.map((metric) => {
          const Icon = metric.icon;
          return (
            <div key={metric.label} className="min-h-28 rounded-lg border border-border bg-card p-4">
              <div className="flex items-center justify-between gap-3">
                <span className="truncate text-xs font-medium text-muted-foreground">
                  {metric.label}
                </span>
                <Icon className="size-4 shrink-0 text-muted-foreground" />
              </div>
              <p className="mt-5 truncate text-xl font-heading font-semibold">{metric.value}</p>
            </div>
          );
        })}
      </section>

      <section className="rounded-lg border border-border bg-card">
        <div className="border-b border-border px-4 py-3">
          <h2 className="text-sm font-heading font-semibold">Module readiness</h2>
        </div>
        <div className="divide-y divide-border">
          {overviewModules.map((module) => (
            <div key={module} className="flex min-h-12 items-center justify-between gap-4 px-4 py-3">
              <span className="text-sm font-medium">{module}</span>
              <span className="shrink-0 rounded-md bg-muted px-2 py-1 text-xs text-muted-foreground">
                Route shell
              </span>
            </div>
          ))}
        </div>
      </section>

      <EnterpriseRouteState
        title="Data integration pending"
        description="This task wires the enterprise admin route structure and durable layout shells. Live tenant metrics and workflows are intentionally outside this slice."
      />
    </div>
  );
}
