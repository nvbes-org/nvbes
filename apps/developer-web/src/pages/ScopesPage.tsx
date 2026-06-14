import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Library } from 'lucide-react';
import { listDeveloperScopes } from '../developer.api';

export function ScopesPage() {
  const scopesQuery = useQuery({
    queryKey: ['developer-scopes'],
    queryFn: ({ signal }) => listDeveloperScopes(signal),
    staleTime: 60_000,
  });

  if (scopesQuery.isLoading) {
    return <div className="h-80 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (scopesQuery.isError || !scopesQuery.data) {
    return <ScopesUnavailable />;
  }

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Library className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Scope registry</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Central catalog of OAuth scopes, risk levels, lifecycle, owners, and audiences.
          </p>
        </div>
      </div>
      <div className="grid gap-3">
        {scopesQuery.data.map((scope) => (
          <article key={scope.scope_key} className="rounded-lg border border-border bg-card p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold">{scope.display_name}</h3>
                <p className="mt-1 font-mono text-xs text-muted-foreground">{scope.scope_key}</p>
              </div>
              <div className="flex gap-2">
                <span className="rounded-md border border-border px-2 py-1 text-xs font-medium capitalize">
                  {scope.risk}
                </span>
                <span className="rounded-md border border-border px-2 py-1 text-xs font-medium capitalize">
                  {scope.lifecycle}
                </span>
              </div>
            </div>
            <p className="mt-3 text-sm text-muted-foreground">{scope.description}</p>
            <p className="mt-3 text-xs text-muted-foreground">
              Owner: {scope.owner_team} · Audiences: {scope.allowed_audiences.join(', ') || 'none'}
            </p>
          </article>
        ))}
        {scopesQuery.data.length === 0 ? (
          <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
            No scopes are registered yet.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function ScopesUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Scope registry unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load the central scope registry.
      </p>
    </section>
  );
}
