import { Activity, Building2, ShieldCheck, Users } from 'lucide-react';
import { EnterpriseRouteState } from '../components/EnterpriseRouteState';
import { Badge } from '../components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';

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
          <Badge variant="outline" className="h-7 shrink-0 rounded-md">
            Shell only
          </Badge>
        </div>
      </header>

      <section className="grid gap-3 md:grid-cols-4">
        {overviewMetrics.map((metric) => {
          const Icon = metric.icon;
          return (
            <Card key={metric.label} className="min-h-28 rounded-lg" size="sm">
              <CardContent>
                <div className="flex items-center justify-between gap-3">
                  <span className="truncate text-xs font-medium text-muted-foreground">
                    {metric.label}
                  </span>
                  <Icon className="size-4 shrink-0 text-muted-foreground" />
                </div>
                <p className="mt-5 truncate text-xl font-heading font-semibold">{metric.value}</p>
              </CardContent>
            </Card>
          );
        })}
      </section>

      <Card className="rounded-lg" size="sm">
        <CardHeader className="border-b border-border">
          <CardTitle>Module readiness</CardTitle>
        </CardHeader>
        <CardContent className="divide-y divide-border px-0">
          {overviewModules.map((module) => (
            <div
              key={module}
              className="flex min-h-12 items-center justify-between gap-4 px-4 py-3"
            >
              <span className="text-sm font-medium">{module}</span>
              <Badge variant="secondary" className="shrink-0 rounded-md">
                Route shell
              </Badge>
            </div>
          ))}
        </CardContent>
      </Card>

      <EnterpriseRouteState
        title="Data integration pending"
        description="This task wires the enterprise admin route structure and durable layout shells. Live tenant metrics and workflows are intentionally outside this slice."
      />
    </div>
  );
}
