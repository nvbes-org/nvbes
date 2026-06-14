import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Activity, AlertTriangle } from 'lucide-react';
import { listDeveloperHealthChecks, runDeveloperHealthChecks } from '../developer.api';
import { countUnhealthyChecks, healthStatusClass } from './HealthChecksPage.helpers';

export function HealthChecksPage() {
  const queryClient = useQueryClient();
  const healthQuery = useQuery({
    queryKey: ['developer-health-checks'],
    queryFn: ({ signal }) => listDeveloperHealthChecks(signal),
    staleTime: 30_000,
  });
  const runMutation = useMutation({
    mutationFn: runDeveloperHealthChecks,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['developer-health-checks'] }),
  });

  if (healthQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (healthQuery.isError || !healthQuery.data) {
    return <HealthUnavailable />;
  }

  const unhealthy = countUnhealthyChecks(healthQuery.data);

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex items-start gap-3">
          <div className="rounded-md border border-border bg-card p-2">
            <Activity className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">Integration health checks</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Redirects, JWKS, OIDC discovery, webhooks, and SCIM checks.
            </p>
          </div>
        </div>
        <button
          className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"
          disabled={runMutation.isPending}
          type="button"
          onClick={() => runMutation.mutate()}
        >
          Run checks
        </button>
      </div>
      <div className="grid gap-3 sm:grid-cols-3">
        <Metric label="Checks" value={healthQuery.data.length.toString()} />
        <Metric label="Unhealthy" value={unhealthy.toString()} />
        <Metric label="Last run" value={healthQuery.data[0]?.checked_at ?? '-'} />
      </div>
      <div className="grid gap-3">
        {healthQuery.data.map((check) => (
          <article key={check.id} className="rounded-lg border border-border bg-card p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold">
                  {check.target_type} / {check.check_kind}
                </h3>
                <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                  {check.target_id}
                </p>
              </div>
              <span
                className={`text-xs font-semibold capitalize ${healthStatusClass(check.status)}`}
              >
                {check.status}
              </span>
            </div>
            <p className="mt-3 text-sm text-muted-foreground">{check.summary}</p>
          </article>
        ))}
        {healthQuery.data.length === 0 ? (
          <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
            No health checks have been run for this tenant.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function HealthUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Health checks unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load health checks.
      </p>
    </section>
  );
}

function Metric(props: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <p className="text-xs font-medium uppercase text-muted-foreground">{props.label}</p>
      <p className="mt-2 break-words text-lg font-semibold">{props.value}</p>
    </div>
  );
}
