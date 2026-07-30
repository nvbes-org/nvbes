import {
  getWebPushSupport,
  requestWebPushPermission,
  type WebPushSupport,
} from '@nvbes/web-runtime';
import { useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { Bell, Mail, Megaphone, Smartphone } from 'lucide-react';
import { useEffect, useState } from 'react';
import { z } from 'zod';
import { accountQueryKeys } from '@/account.queries';
import { AccountPage, AccountPageHeader } from '@/components/AccountPage';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from '@/components/ui/item';
import { useAuthuser } from '@/hooks/useAuthuser';
import { identityHttpClient } from '../identity.http';

const NotificationsSchema = z.object({
  email: z.boolean(),
  push: z.boolean(),
  in_app: z.boolean(),
  marketing_email: z.boolean(),
});

type NotificationPrefs = z.infer<typeof NotificationsSchema>;

function pushDescription(support: WebPushSupport): string {
  if (support.supported && support.permission === 'granted') {
    return 'Notifications push autorisees dans ce navigateur.';
  }

  if (support.supported && support.permission === 'denied') {
    return 'Notifications bloquees dans les reglages du navigateur.';
  }

  if (support.supported) {
    return 'Notifications push dans le navigateur.';
  }

  if (support.reason === 'missing-push-manager') {
    return "Indisponible dans ce contexte. Sur iPhone ou iPad, ajoutez l'app a l'ecran d'accueil.";
  }

  return 'Notifications push indisponibles dans ce navigateur.';
}

function pushButtonLabel(enabled: boolean, support: WebPushSupport): string {
  if (!support.supported) return 'Indisponible';
  if (support.permission === 'denied') return 'Bloque';
  return enabled ? 'Active' : 'Desactive';
}

function canTogglePush(support: WebPushSupport): boolean {
  return support.supported && support.permission !== 'denied';
}

function fetchNotifications() {
  return identityHttpClient.request('/auth/me/notifications', NotificationsSchema, {
    method: 'GET',
  });
}

function updateNotifications(prefs: NotificationPrefs) {
  return identityHttpClient.request('/auth/me/notifications', NotificationsSchema, {
    method: 'PUT',
    body: prefs,
  });
}

export function getAccountNotificationsQueryKey(
  authuser: string,
): readonly ['identity', 'account', string, 'notifications'] {
  return [...accountQueryKeys.byAuthuser(authuser), 'notifications'] as const;
}

export default function AccountNotificationsPage() {
  const authuser = useAuthuser();
  const queryClient = useQueryClient();
  const notificationsQueryKey = getAccountNotificationsQueryKey(authuser);
  const [webPushSupport, setWebPushSupport] = useState<WebPushSupport>(() => getWebPushSupport());

  useEffect(() => {
    const refreshSupport = () => setWebPushSupport(getWebPushSupport());
    window.addEventListener('focus', refreshSupport);
    document.addEventListener('visibilitychange', refreshSupport);
    return () => {
      window.removeEventListener('focus', refreshSupport);
      document.removeEventListener('visibilitychange', refreshSupport);
    };
  }, []);

  const { data } = useSuspenseQuery({
    queryKey: notificationsQueryKey,
    queryFn: fetchNotifications,
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const mutation = useMutation({
    mutationFn: (prefs: NotificationPrefs) => updateNotifications(prefs),
    onSuccess: () => {
      void queryClient.invalidateQueries({ exact: true, queryKey: notificationsQueryKey });
    },
  });

  const prefs = data;

  const togglePref = async (key: keyof NotificationPrefs) => {
    if (key === 'push' && !prefs.push) {
      const permission = await requestWebPushPermission();
      setWebPushSupport(getWebPushSupport());
      if (permission !== 'granted') return;
    }

    mutation.mutate({ ...prefs, [key]: !prefs[key] });
  };

  const channels = [
    {
      key: 'email' as const,
      label: 'Email',
      desc: 'Notifications par email pour les evenements importants.',
      Icon: Mail,
    },
    {
      key: 'push' as const,
      label: 'Push',
      desc: pushDescription(webPushSupport),
      Icon: Smartphone,
    },
    {
      key: 'in_app' as const,
      label: 'In-app',
      desc: "Notifications dans l'application.",
      Icon: Bell,
    },
    {
      key: 'marketing_email' as const,
      label: 'Nouveautes',
      desc: 'Conseils produit et nouveautes par email.',
      Icon: Megaphone,
    },
  ];

  return (
    <AccountPage>
      <AccountPageHeader
        size="section"
        title="Notifications"
        description="Gerer vos preferences de notification."
      />

      <Card>
        <CardHeader>
          <CardTitle>Canaux de notification</CardTitle>
          <CardDescription>
            Choisissez comment vous souhaitez recevoir les notifications.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-1">
          {channels.map(({ key, label, desc, Icon }, i) => (
            <div key={key}>
              {i > 0 && <Separator className="my-1" />}
              <Item className="gap-3 px-0 py-1">
                <ItemMedia variant="icon" className="size-8 rounded-lg bg-muted">
                  <Icon className="size-4 text-muted-foreground" />
                </ItemMedia>
                <ItemContent className="min-w-0 gap-0">
                  <ItemTitle>{label}</ItemTitle>
                  <ItemDescription className="text-xs">{desc}</ItemDescription>
                </ItemContent>
                <ItemActions className="shrink-0">
                  <Button
                    variant={prefs[key] ? 'default' : 'outline'}
                    size="sm"
                    className="shrink-0"
                    onClick={() => void togglePref(key)}
                    disabled={
                      mutation.isPending || (key === 'push' && !canTogglePush(webPushSupport))
                    }
                  >
                    {key === 'push'
                      ? pushButtonLabel(prefs.push, webPushSupport)
                      : prefs[key]
                        ? 'Active'
                        : 'Desactive'}
                  </Button>
                </ItemActions>
              </Item>
            </div>
          ))}
        </CardContent>
      </Card>
    </AccountPage>
  );
}
