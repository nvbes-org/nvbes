import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Bot } from 'lucide-react';
import { listDeveloperServiceAccounts } from '../developer.api';

export function ServiceAccountsPage() {
  const serviceAccountsQuery = useQuery({
    queryKey: ['developer-service-accounts'],
    queryFn: ({ signal }) => listDeveloperServiceAccounts(signal),
    staleTime: 30_000,
  });

  if (serviceAccountsQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (serviceAccountsQuery.isError || !serviceAccountsQuery.data) {
    return <ServiceAccountsUnavailable />;
  }

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Bot className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Service accounts</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Tenant service identities and their attached OAuth client count.
          </p>
        </div>
      </div>
      <div className="grid gap-3">
        {serviceAccountsQuery.data.map((account) => (
          <article
            key={account.principal_id}
            className="rounded-lg border border-border bg-card p-4"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold">{account.name}</h3>
                <p className="mt-1 font-mono text-xs text-muted-foreground">
                  {account.principal_id}
                </p>
              </div>
              <span className="rounded-md border border-border px-2 py-1 text-xs font-medium capitalize">
                {account.status}
              </span>
            </div>
            <p className="mt-3 text-sm text-muted-foreground">
              Role: {account.role} · OAuth clients: {account.oauth_client_count}
            </p>
          </article>
        ))}
        {serviceAccountsQuery.data.length === 0 ? (
          <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
            No service accounts are registered for this tenant.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function ServiceAccountsUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Service accounts unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load tenant service accounts.
      </p>
    </section>
  );
}
