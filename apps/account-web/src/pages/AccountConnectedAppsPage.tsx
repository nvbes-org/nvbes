import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { accountIdentityClient } from '@/account.identity';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

const clientsKey = ['account', 'identity', 'oauth-clients'] as const;

export default function AccountConnectedAppsPage() {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: clientsKey,
    queryFn: () => accountIdentityClient().listOAuthClients(),
  });
  const revoke = useMutation({
    mutationFn: (id: string) => accountIdentityClient().revokeOAuthClient(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: clientsKey }),
  });
  return (
    <AccountPage
      title="Applications connectées"
      description="Consultez les clients OAuth autorisés et révoquez les accès devenus inutiles."
    >
      {query.isPending ? (
        <AccountPageLoading label="Chargement des applications…" />
      ) : query.error || !query.data ? (
        <AccountPageError error={query.error} onRetry={() => void query.refetch()} />
      ) : query.data.length === 0 ? (
        <p className="text-sm text-muted-foreground">Aucune application connectée.</p>
      ) : (
        <div className="divide-y divide-border border-y border-border">
          {query.data.map((client) => (
            <section
              key={client.client_id}
              className="flex items-center justify-between gap-4 py-4"
            >
              <div>
                <p className="text-sm font-medium">{client.name}</p>
                <p className="text-xs text-muted-foreground">{client.client_id}</p>
              </div>
              <Button
                type="button"
                variant="destructive"
                disabled={revoke.isPending}
                onClick={() => revoke.mutate(client.client_id)}
              >
                Révoquer
              </Button>
            </section>
          ))}
        </div>
      )}
      {revoke.error ? (
        <Alert variant="destructive">
          <AlertDescription>{revoke.error.message}</AlertDescription>
        </Alert>
      ) : null}
    </AccountPage>
  );
}
