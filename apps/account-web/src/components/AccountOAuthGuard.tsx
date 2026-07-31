import { useLocation } from '@tanstack/react-router';
import { useEffect, useState, useSyncExternalStore } from 'react';
import {
  getAccountAccessTokenSnapshot,
  subscribeAccountAccessToken,
} from '@/account.oauth.access-token';
import { startAccountAuthorization } from '@/account.oauth.client';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';

export function AccountOAuthGuard({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const accessToken = useSyncExternalStore(
    subscribeAccountAccessToken,
    getAccountAccessTokenSnapshot,
    () => null,
  );
  const [error, setError] = useState<string | null>(null);
  const returnTo = `${location.pathname}${location.searchStr}${location.hash}`;

  useEffect(() => {
    if (accessToken) {
      setError(null);
      return;
    }

    void startAccountAuthorization(returnTo).catch((reason: unknown) => {
      setError(reason instanceof Error ? reason.message : 'Identity est indisponible.');
    });
  }, [accessToken, returnTo]);

  if (accessToken) {
    return children;
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-lg items-center justify-center px-6">
      {error ? (
        <Alert variant="destructive">
          <AlertTitle>Connexion impossible</AlertTitle>
          <AlertDescription className="flex flex-col gap-4">
            <span>{error}</span>
            <Button type="button" variant="outline" onClick={() => window.location.reload()}>
              Réessayer
            </Button>
          </AlertDescription>
        </Alert>
      ) : (
        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <Spinner />
          Redirection sécurisée vers Identity…
        </div>
      )}
    </main>
  );
}
