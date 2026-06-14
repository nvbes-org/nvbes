import { useQuery } from '@tanstack/react-query';
import { Activity, AlertTriangle, Boxes, KeyRound, Library, Store, Webhook } from 'lucide-react';
import { getDeveloperOverview } from '../developer.api';
import type { DeveloperOverview } from '../developer.schemas';

type Metric = {
  label: string;
  value: number;
  tone: 'neutral' | 'attention' | 'danger';
  icon: React.ComponentType<{ className?: string }>;
};

function buildMetrics(overview: DeveloperOverview): Metric[] {
  return [
    { label: 'OAuth clients', value: overview.oauthClients, tone: 'neutral', icon: KeyRound },
    {
      label: 'Pending approvals',
      value: overview.marketplacePending,
      tone: 'attention',
      icon: Store,
    },
    { label: 'High-risk scopes', value: overview.highRiskScopes, tone: 'attention', icon: Library },
    {
      label: 'Failed webhook deliveries',
      value: overview.failedWebhookDeliveries,
      tone: 'danger',
      icon: Webhook,
    },
    {
      label: 'Unhealthy integrations',
      value: overview.unhealthyIntegrations,
      tone: 'danger',
      icon: AlertTriangle,
    },
    { label: 'Active sandboxes', value: overview.activeSandboxes, tone: 'neutral', icon: Boxes },
  ];
}

const toneClasses: Record<Metric['tone'], string> = {
  neutral: 'text-primary',
  attention: 'text-amber-600',
  danger: 'text-red-600',
};

export function OverviewPage() {
  const overviewQuery = useQuery({
    queryKey: ['developer-overview'],
    queryFn: ({ signal }) => getDeveloperOverview(signal),
    staleTime: 30_000,
  });

  if (overviewQuery.isLoading) {
    return <OverviewSkeleton />;
  }

  if (overviewQuery.isError || !overviewQuery.data) {
    return (
      <section className="rounded-lg border border-border bg-card p-6">
        <div className="flex items-center gap-3 text-red-600">
          <AlertTriangle className="h-5 w-5" />
          <h2 className="text-base font-semibold">Developer context unavailable</h2>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          The console could not load tenant-scoped integration data.
        </p>
      </section>
    );
  }

  const metrics = buildMetrics(overviewQuery.data);

  return (
    <div className="space-y-6">
      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {metrics.map((metric) => {
          const Icon = metric.icon;

          return (
            <article key={metric.label} className="rounded-lg border border-border bg-card p-5">
              <div className="flex items-center justify-between gap-4">
                <p className="text-sm font-medium text-muted-foreground">{metric.label}</p>
                <Icon className={`h-5 w-5 ${toneClasses[metric.tone]}`} />
              </div>
              <p className="mt-4 text-3xl font-semibold tracking-normal">{metric.value}</p>
            </article>
          );
        })}
      </section>

      <section className="grid gap-4 xl:grid-cols-[1fr_360px]">
        <article className="rounded-lg border border-border bg-card p-5">
          <div className="flex items-center gap-3">
            <Activity className="h-5 w-5 text-primary" />
            <h2 className="text-base font-semibold">Integration readiness</h2>
          </div>
          <div className="mt-5 grid gap-3">
            {[
              ['Redirect URI coverage', 'OAuth clients expose testable redirect policies.'],
              ['OIDC discovery', 'JWKS and discovery checks are ready for automated runs.'],
              ['Webhook recovery', 'Failed deliveries can be isolated before replay.'],
            ].map(([title, description]) => (
              <div key={title} className="rounded-md border border-border bg-background px-4 py-3">
                <p className="text-sm font-medium">{title}</p>
                <p className="mt-1 text-sm text-muted-foreground">{description}</p>
              </div>
            ))}
          </div>
        </article>

        <article className="rounded-lg border border-border bg-card p-5">
          <h2 className="text-base font-semibold">Tenant</h2>
          <p className="mt-2 break-all text-sm text-muted-foreground">
            {overviewQuery.data.tenantId}
          </p>
        </article>
      </section>
    </div>
  );
}

function OverviewSkeleton() {
  return (
    <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      {['clients', 'marketplace', 'scopes', 'webhooks', 'health', 'sandboxes'].map((item) => (
        <div key={item} className="h-32 animate-pulse rounded-lg border border-border bg-card" />
      ))}
    </section>
  );
}
