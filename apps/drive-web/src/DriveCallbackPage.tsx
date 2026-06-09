import { AuthErrorBoundary, clientErrorMessage } from '@nvbes/web-runtime';
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { completeDriveCallback } from './drive.auth.functions';

export function DriveCallbackPage() {
  return (
    <AuthErrorBoundary>
      <CallbackContent />
    </AuthErrorBoundary>
  );
}

let callbackExchangeStarted = false;

function CallbackContent() {
  const navigate = useNavigate();
  const navigateRef = useRef(navigate);
  navigateRef.current = navigate;
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (callbackExchangeStarted) return;
    callbackExchangeStarted = true;

    let cancelled = false;
    const searchParams = new URLSearchParams(window.location.search);

    async function run() {
      try {
        await completeDriveCallback(searchParams);
        void navigateRef.current({ to: '/', replace: true });
      } catch (err) {
        if (!cancelled) {
          setError(clientErrorMessage(err, 'OAuth callback failed.'));
        }
      }
    }

    void run();

    return () => {
      cancelled = true;
      setTimeout(() => {
        callbackExchangeStarted = false;
      }, 0);
    };
    // Run once on mount; navigate ref avoids stale closure & dep instability
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (error) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(15,23,42,0.08),_transparent_35%),linear-gradient(180deg,_#fbfcfe_0%,_#f3f6fb_100%)] px-4">
        <Card className="w-full max-w-md">
          <CardHeader>
            <p className="text-sm font-semibold uppercase tracking-[0.28em] text-muted-foreground">
              Authentification
            </p>
            <CardTitle className="text-3xl">Impossible de terminer la connexion</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-6">
            <Alert variant="destructive">
              <AlertTitle>Erreur OAuth</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
            <div className="flex items-center gap-3">
              <Button asChild>
                <a href="/">Retourner à Drive</a>
              </Button>
              <Button asChild variant="outline">
                <a href="mailto:support@nvbes.fr?subject=Erreur%20Drive%20callback">Signaler</a>
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(15,23,42,0.08),_transparent_35%),linear-gradient(180deg,_#fbfcfe_0%,_#f3f6fb_100%)] px-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <p className="text-sm font-semibold uppercase tracking-[0.28em] text-muted-foreground">
            Authentification
          </p>
          <CardTitle className="text-3xl">Connexion en cours</CardTitle>
        </CardHeader>
        <CardContent className="text-sm leading-6 text-muted-foreground">
          Nous validons votre session Identity et préparons votre espace Drive.
        </CardContent>
      </Card>
    </div>
  );
}
