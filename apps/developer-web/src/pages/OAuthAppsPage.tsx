import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, CheckCircle2, KeyRound, UploadCloud } from 'lucide-react';
import {
  getDeveloperContext,
  listDeveloperOAuthClients,
  submitDeveloperMarketplaceApp,
} from '../developer.api';
import { canUseDeveloperPermission } from '../developer.permissions';
import type { DeveloperOAuthClient } from '../developer.schemas';

export function OAuthAppsPage() {
  const clientsQuery = useQuery({
    queryKey: ['developer-oauth-clients'],
    queryFn: ({ signal }) => listDeveloperOAuthClients(signal),
    staleTime: 30_000,
  });

  if (clientsQuery.isLoading) {
    return <TableSkeleton />;
  }

  if (clientsQuery.isError || !clientsQuery.data) {
    return <UnavailableState label="OAuth clients unavailable" />;
  }

  return (
    <section className="space-y-4">
      <PageHeader
        icon={KeyRound}
        title="OAuth clients"
        description="Review app registration, consent readiness, scopes, and latest health status."
      />
      <div className="overflow-hidden rounded-lg border border-border bg-card">
        <table className="w-full min-w-[760px] text-left text-sm">
          <thead className="border-b border-border bg-muted/60 text-xs uppercase text-muted-foreground">
            <tr>
              <th className="px-4 py-3 font-medium">Client</th>
              <th className="px-4 py-3 font-medium">Marketplace</th>
              <th className="px-4 py-3 font-medium">Consent</th>
              <th className="px-4 py-3 font-medium">Redirects</th>
              <th className="px-4 py-3 font-medium">Scopes</th>
              <th className="px-4 py-3 font-medium">Health</th>
            </tr>
          </thead>
          <tbody>
            {clientsQuery.data.map((client) => (
              <OAuthClientRow key={client.client_id} client={client} />
            ))}
          </tbody>
        </table>
        {clientsQuery.data.length === 0 ? <EmptyState label="No OAuth clients registered" /> : null}
      </div>
    </section>
  );
}

function OAuthClientRow({ client }: { client: DeveloperOAuthClient }) {
  const queryClient = useQueryClient();
  const contextQuery = useQuery({
    queryKey: ['developer-context'],
    queryFn: ({ signal }) => getDeveloperContext(signal),
    staleTime: 60_000,
  });

  const submitMutation = useMutation({
    mutationFn: (clientId: string) => submitDeveloperMarketplaceApp(clientId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['developer-oauth-clients'] });
      void queryClient.invalidateQueries({ queryKey: ['developer-marketplace-apps'] });
      void queryClient.invalidateQueries({ queryKey: ['developer-overview'] });
    },
  });

  const canSubmit =
    contextQuery.data &&
    canUseDeveloperPermission(contextQuery.data, 'marketplace.submit') &&
    (!client.marketplace_status ||
      client.marketplace_status === 'rejected' ||
      client.marketplace_status === 'suspended');

  return (
    <tr className="border-b border-border last:border-0 hover:bg-muted/30">
      <td className="px-4 py-3">
        <p className="font-medium">{client.name}</p>
        <p className="mt-1 font-mono text-xs text-muted-foreground">{client.client_id}</p>
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-3">
          <span className="capitalize">{client.marketplace_status ?? 'not submitted'}</span>
          {canSubmit ? (
            <button
              onClick={() => submitMutation.mutate(client.client_id)}
              disabled={submitMutation.isPending}
              className="inline-flex items-center gap-1 rounded border border-border bg-background px-2 py-0.5 text-xs font-medium text-foreground hover:bg-muted disabled:opacity-50 transition-colors"
            >
              <UploadCloud className="h-3 w-3" />
              {submitMutation.isPending ? 'Submitting...' : 'Submit'}
            </button>
          ) : null}
        </div>
      </td>
      <td className="px-4 py-3">
        <span className="inline-flex items-center gap-2">
          {client.consent_screen_configured ? (
            <CheckCircle2 className="h-4 w-4 text-primary" />
          ) : (
            <AlertTriangle className="h-4 w-4 text-amber-600" />
          )}
          {client.consent_screen_configured ? 'Configured' : 'Missing'}
        </span>
      </td>
      <td className="px-4 py-3">{client.redirect_uri_count}</td>
      <td className="px-4 py-3">{client.allowed_scopes.length}</td>
      <td className="px-4 py-3 capitalize">{client.health_status}</td>
    </tr>
  );
}

function PageHeader({
  icon: Icon,
  title,
  description,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
}) {
  return (
    <div className="flex items-start gap-3">
      <div className="rounded-md border border-border bg-card p-2">
        <Icon className="h-5 w-5 text-primary" />
      </div>
      <div>
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

function TableSkeleton() {
  return <div className="h-80 animate-pulse rounded-lg border border-border bg-card" />;
}

function EmptyState({ label }: { label: string }) {
  return <p className="border-t border-border px-4 py-6 text-sm text-muted-foreground">{label}</p>;
}

function UnavailableState({ label }: { label: string }) {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">{label}</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load tenant-scoped integration data.
      </p>
    </section>
  );
}
