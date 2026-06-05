import { useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { Bell, Mail, Smartphone } from 'lucide-react';
import { z } from 'zod';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { identityHttpClient } from '../identity.http';

const NotificationsSchema = z.object({
  email: z.boolean(),
  push: z.boolean(),
  in_app: z.boolean(),
});

type NotificationPrefs = z.infer<typeof NotificationsSchema>;

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

export default function AccountNotificationsPage() {
  const queryClient = useQueryClient();

  const { data } = useSuspenseQuery({
    queryKey: ['notifications'],
    queryFn: fetchNotifications,
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const mutation = useMutation({
    mutationFn: (prefs: NotificationPrefs) => updateNotifications(prefs),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['notifications'] });
    },
  });

  const prefs = data;

  const togglePref = (key: keyof NotificationPrefs) => {
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
      desc: 'Notifications push dans le navigateur.',
      Icon: Smartphone,
    },
    {
      key: 'in_app' as const,
      label: 'In-app',
      desc: "Notifications dans l'application.",
      Icon: Bell,
    },
  ];

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Notifications</h1>
        <p className="text-sm text-muted-foreground mt-1">Gerer vos preferences de notification.</p>
      </div>

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
              <div className="flex items-center justify-between gap-3 py-1">
                <div className="flex items-center gap-3 min-w-0">
                  <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                    <Icon className="size-4 text-muted-foreground" />
                  </div>
                  <div className="flex flex-col min-w-0">
                    <span className="text-sm font-medium">{label}</span>
                    <span className="text-xs text-muted-foreground">{desc}</span>
                  </div>
                </div>
                <Button
                  variant={prefs[key] ? 'default' : 'outline'}
                  size="sm"
                  className="shrink-0"
                  onClick={() => togglePref(key)}
                  disabled={mutation.isPending}
                >
                  {prefs[key] ? 'Active' : 'Desactive'}
                </Button>
              </div>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
