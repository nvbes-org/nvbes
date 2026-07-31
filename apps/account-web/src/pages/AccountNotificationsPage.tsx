import type { AccountNotifications } from '@nvbes/account-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { accountClient } from '@/account.client';
import { useAccountAuthenticationRecovery } from '@/account.authentication';
import { accountQueryKeys } from '@/account.queries';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';

const notificationOptions = [
  {
    key: 'email',
    label: 'Notifications par e-mail',
    description: 'Recevoir les alertes opérationnelles importantes.',
  },
  {
    key: 'push',
    label: 'Notifications push',
    description: 'Recevoir des alertes sur vos appareils autorisés.',
  },
  {
    key: 'in_app',
    label: 'Notifications dans les applications',
    description: 'Afficher les nouveautés directement dans nvbes.',
  },
  {
    key: 'marketing_email',
    label: 'Actualités produit',
    description: 'Recevoir les annonces et conseils d’utilisation.',
  },
] as const satisfies ReadonlyArray<{
  key: keyof AccountNotifications;
  label: string;
  description: string;
}>;

export default function AccountNotificationsPage() {
  const queryClient = useQueryClient();
  const notificationsQuery = useQuery({
    queryKey: accountQueryKeys.notifications,
    queryFn: ({ signal }) => accountClient.getNotifications({ signal }),
  });
  const [draft, setDraft] = useState<AccountNotifications | null>(null);
  const [saved, setSaved] = useState(false);

  useAccountAuthenticationRecovery(notificationsQuery.error);

  useEffect(() => {
    if (notificationsQuery.data) {
      setDraft(notificationsQuery.data);
    }
  }, [notificationsQuery.data]);

  const updateNotifications = useMutation({
    mutationFn: (notifications: AccountNotifications) =>
      accountClient.updateNotifications(notifications),
    onSuccess: (notifications) => {
      queryClient.setQueryData(accountQueryKeys.notifications, notifications);
      setDraft(notifications);
      setSaved(true);
    },
  });
  useAccountAuthenticationRecovery(updateNotifications.error);

  return (
    <AccountPage
      title="Notifications"
      description="Choisissez les canaux sur lesquels Account peut vous contacter."
    >
      {notificationsQuery.isPending ? (
        <AccountPageLoading label="Chargement des notifications…" />
      ) : notificationsQuery.error || !draft ? (
        <AccountPageError
          error={notificationsQuery.error}
          onRetry={() => void notificationsQuery.refetch()}
        />
      ) : (
        <div className="space-y-6">
          <div className="divide-y divide-border border-y border-border">
            {notificationOptions.map(({ key, label, description }) => (
              <label
                key={key}
                className="flex cursor-pointer items-center justify-between gap-5 py-5"
              >
                <span>
                  <span className="block text-sm font-medium text-foreground">{label}</span>
                  <span className="mt-1 block text-sm text-muted-foreground">{description}</span>
                </span>
                <Switch
                  checked={draft[key]}
                  aria-label={label}
                  onCheckedChange={(checked) => {
                    setSaved(false);
                    setDraft((current) => (current ? { ...current, [key]: checked } : current));
                  }}
                />
              </label>
            ))}
          </div>

          {updateNotifications.error ? (
            <Alert variant="destructive">
              <AlertTitle>Enregistrement impossible</AlertTitle>
              <AlertDescription>{updateNotifications.error.message}</AlertDescription>
            </Alert>
          ) : null}
          {saved ? (
            <Alert>
              <AlertTitle>Notifications enregistrées</AlertTitle>
              <AlertDescription>Vos canaux de contact ont été mis à jour.</AlertDescription>
            </Alert>
          ) : null}

          <Button
            type="button"
            disabled={updateNotifications.isPending}
            onClick={() => {
              setSaved(false);
              updateNotifications.mutate(draft);
            }}
          >
            {updateNotifications.isPending ? 'Enregistrement…' : 'Enregistrer'}
          </Button>
        </div>
      )}
    </AccountPage>
  );
}
