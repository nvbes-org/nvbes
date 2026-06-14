import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Store } from 'lucide-react';
import { listDeveloperMarketplaceApps } from '../developer.api';

export function MarketplacePage() {
  const appsQuery = useQuery({
    queryKey: ['developer-marketplace-apps'],
    queryFn: ({ signal }) => listDeveloperMarketplaceApps(signal),
    staleTime: 30_000,
  });

  if (appsQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (appsQuery.isError || !appsQuery.data) {
    return <MarketplaceUnavailable />;
  }

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Store className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">OAuth App Marketplace</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Track OAuth apps awaiting approval or already authorized for internal use.
          </p>
        </div>
      </div>
      <div className="grid gap-3">
        {appsQuery.data.map((app) => (
          <article key={app.client_id} className="rounded-lg border border-border bg-card p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold">{app.name}</h3>
                <p className="mt-1 font-mono text-xs text-muted-foreground">{app.client_id}</p>
              </div>
              <span className="rounded-md border border-border px-2 py-1 text-xs font-medium capitalize">
                {app.status}
              </span>
            </div>
            {app.review_reason ? (
              <p className="mt-3 text-sm text-muted-foreground">{app.review_reason}</p>
            ) : null}
          </article>
        ))}
        {appsQuery.data.length === 0 ? (
          <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
            No OAuth apps are currently submitted to the marketplace.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function MarketplaceUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Marketplace unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load marketplace approvals.
      </p>
    </section>
  );
}
