import { useEffect, useState } from 'react';
import { completeAccountOAuthCallback } from '@/account.oauth.callback';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';

type CallbackStatus = { kind: 'loading' } | { kind: 'error'; message: string };

export default function AccountOAuthCallbackPage() {
  const [status, setStatus] = useState<CallbackStatus>({ kind: 'loading' });

  useEffect(() => {
    let active = true;

    completeAccountOAuthCallback()
      .then((returnTo) => {
        if (active) {
          window.location.replace(returnTo);
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setStatus({
            kind: 'error',
            message: error instanceof Error ? error.message : 'La connexion OAuth a échoué.',
          });
        }
      });

    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-lg items-center justify-center px-6">
      {status.kind === 'loading' ? (
        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <Spinner />
          Connexion sécurisée au compte…
        </div>
      ) : (
        <Alert variant="destructive">
          <AlertTitle>Connexion impossible</AlertTitle>
          <AlertDescription className="flex flex-col gap-4">
            <span>{status.message}</span>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                window.location.replace('/');
              }}
            >
              Réessayer
            </Button>
          </AlertDescription>
        </Alert>
      )}
    </main>
  );
}
