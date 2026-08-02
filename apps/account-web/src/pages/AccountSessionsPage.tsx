import type { AccountSession } from '@nvbes/account-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { accountClient } from '@/account.client';
import { useAccountAuthenticationRecovery } from '@/account.authentication';
import { accountQueryKeys } from '@/account.queries';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Button } from '@/components/ui/button';

export default function AccountSessionsPage() {
  const queryClient = useQueryClient();
  const sessionsQuery = useQuery({
    queryKey: accountQueryKeys.sessions,
    queryFn: ({ signal }) => accountClient.listSessions({ limit: 100, signal }),
  });
  const revokeSession = useMutation({
    mutationFn: (sessionId: string) => accountClient.revokeSession(sessionId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: accountQueryKeys.sessions }),
  });

  useAccountAuthenticationRecovery(sessionsQuery.error);
  useAccountAuthenticationRecovery(revokeSession.error);

  return (
    <AccountPage
      title="Sessions actives"
      description="Consultez les navigateurs connectés à votre compte et révoquez ceux que vous ne reconnaissez pas."
    >
      {sessionsQuery.isPending ? (
        <AccountPageLoading label="Chargement des sessions…" />
      ) : sessionsQuery.error || !sessionsQuery.data ? (
        <AccountPageError
          error={sessionsQuery.error}
          onRetry={() => void sessionsQuery.refetch()}
        />
      ) : sessionsQuery.data.sessions.length === 0 ? (
        <p className="text-sm text-muted-foreground">Aucune session active.</p>
      ) : (
        <div className="divide-y divide-border border-y border-border">
          {sessionsQuery.data.sessions.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              revoking={revokeSession.isPending && revokeSession.variables === session.id}
              onRevoke={() => revokeSession.mutate(session.id)}
            />
          ))}
        </div>
      )}
    </AccountPage>
  );
}

function SessionRow({
  session,
  revoking,
  onRevoke,
}: {
  session: AccountSession;
  revoking: boolean;
  onRevoke: () => void;
}) {
  const client = [session.client?.browser, session.client?.os].filter(Boolean).join(' · ');
  return (
    <section className="flex items-start justify-between gap-4 py-5">
      <div className="min-w-0 space-y-1">
        <h2 className="text-sm font-semibold text-foreground">
          {client || session.user_agent || 'Appareil inconnu'}
        </h2>
        <p className="text-xs text-muted-foreground">
          Dernière activité {new Date(session.last_seen_at).toLocaleString('fr-FR')}
        </p>
        <p className="text-xs text-muted-foreground">
          {[session.ip, session.geo_country_code].filter(Boolean).join(' · ') ||
            'Localisation indisponible'}
        </p>
        {session.current ? (
          <p className="text-xs font-medium text-primary">Session actuelle</p>
        ) : null}
      </div>
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={session.current || revoking}
        onClick={onRevoke}
      >
        {revoking ? 'Révocation…' : 'Révoquer'}
      </Button>
    </section>
  );
}
