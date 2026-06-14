import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Webhook } from 'lucide-react';
import { listDeveloperWebhooks } from '../developer.api';

export function WebhooksPage() {
  const webhooksQuery = useQuery({
    queryKey: ['developer-webhooks'],
    queryFn: ({ signal }) => listDeveloperWebhooks(signal),
    staleTime: 30_000,
  });

  if (webhooksQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (webhooksQuery.isError || !webhooksQuery.data) {
    return <WebhooksUnavailable />;
  }

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Webhook className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Webhooks</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Monitor endpoints and failed deliveries eligible for replay.
          </p>
        </div>
      </div>
      <div className="grid gap-3">
        {webhooksQuery.data.map((endpoint) => (
          <article key={endpoint.id} className="rounded-lg border border-border bg-card p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold">{endpoint.name}</h3>
                <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                  {endpoint.url}
                </p>
              </div>
              <span className="rounded-md border border-border px-2 py-1 text-xs font-medium capitalize">
                {endpoint.status}
              </span>
            </div>
            <p className="mt-3 text-sm text-muted-foreground">
              Failed deliveries: {endpoint.failed_delivery_count}
            </p>
          </article>
        ))}
        {webhooksQuery.data.length === 0 ? (
          <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
            No webhook endpoints are configured for this tenant.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function WebhooksUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Webhooks unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load webhook endpoints.
      </p>
    </section>
  );
}
