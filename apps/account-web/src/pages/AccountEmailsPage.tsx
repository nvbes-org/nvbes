import type { EmailAddress } from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type FormEvent } from 'react';
import { accountIdentityClient } from '@/account.identity';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const emailsKey = ['account', 'identity', 'emails'] as const;

export default function AccountEmailsPage() {
  const queryClient = useQueryClient();
  const [email, setEmail] = useState('');
  const query = useQuery({
    queryKey: emailsKey,
    queryFn: () => accountIdentityClient().listEmailsPage({ limit: 200 }),
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: emailsKey });
  const add = useMutation({
    mutationFn: () => accountIdentityClient().addSecondaryEmail(email),
    onSuccess: async () => {
      setEmail('');
      await refresh();
    },
  });
  const promote = useMutation({
    mutationFn: (id: string) => accountIdentityClient().promoteSecondaryEmail(id),
    onSuccess: refresh,
  });
  const resend = useMutation({
    mutationFn: (id: string) => accountIdentityClient().resendSecondaryEmailVerification(id),
    onSuccess: refresh,
  });
  const remove = useMutation({
    mutationFn: (id: string) => accountIdentityClient().deleteSecondaryEmail(id),
    onSuccess: refresh,
  });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (email) add.mutate();
  };
  return (
    <AccountPage
      title="Adresses e-mail"
      description="Gérez l’adresse principale et les adresses secondaires utilisées par votre compte."
    >
      <form className="flex max-w-xl items-end gap-2" onSubmit={submit}>
        <div className="flex-1 space-y-2">
          <Label htmlFor="secondary-email">Nouvelle adresse</Label>
          <Input
            id="secondary-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            autoComplete="email"
          />
        </div>
        <Button type="submit" disabled={!email || add.isPending}>
          Ajouter
        </Button>
      </form>
      {add.error ? (
        <Alert variant="destructive">
          <AlertDescription>{add.error.message}</AlertDescription>
        </Alert>
      ) : null}
      {query.isPending ? (
        <AccountPageLoading label="Chargement des adresses…" />
      ) : query.error || !query.data ? (
        <AccountPageError error={query.error} onRetry={() => void query.refetch()} />
      ) : (
        <div className="divide-y divide-border border-y border-border">
          {query.data.emails.map((item) => (
            <EmailRow
              key={item.id}
              email={item}
              busy={promote.isPending || resend.isPending || remove.isPending}
              onPromote={() => promote.mutate(item.id)}
              onResend={() => resend.mutate(item.id)}
              onRemove={() => remove.mutate(item.id)}
            />
          ))}
        </div>
      )}
      {[promote.error, resend.error, remove.error].filter(Boolean).map((error) => (
        <Alert key={error?.message} variant="destructive">
          <AlertDescription>{error?.message}</AlertDescription>
        </Alert>
      ))}
    </AccountPage>
  );
}

function EmailRow({
  email,
  busy,
  onPromote,
  onResend,
  onRemove,
}: {
  email: EmailAddress;
  busy: boolean;
  onPromote: () => void;
  onResend: () => void;
  onRemove: () => void;
}) {
  return (
    <section className="flex flex-wrap items-center justify-between gap-3 py-4">
      <div>
        <p className="text-sm font-medium">{email.email}</p>
        <p className="text-xs text-muted-foreground">
          {email.is_primary ? 'Adresse principale' : 'Adresse secondaire'} ·{' '}
          {email.verified ? 'Vérifiée' : 'Non vérifiée'}
        </p>
      </div>
      <div className="flex flex-wrap gap-2">
        {!email.verified ? (
          <Button type="button" variant="outline" disabled={busy} onClick={onResend}>
            Renvoyer la vérification
          </Button>
        ) : null}
        {!email.is_primary && email.verified ? (
          <Button type="button" variant="outline" disabled={busy} onClick={onPromote}>
            Définir comme principale
          </Button>
        ) : null}
        {!email.is_primary ? (
          <Button type="button" variant="destructive" disabled={busy} onClick={onRemove}>
            Supprimer
          </Button>
        ) : null}
      </div>
    </section>
  );
}
