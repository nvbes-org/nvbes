import { AuthErrorBoundary, clientErrorMessage, runClientEffect } from '@nvbes/web-runtime';
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import { completeDriveCallbackWorkflow } from './drive.workflow';

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
        await runClientEffect(completeDriveCallbackWorkflow(searchParams));
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
        <div className="w-full max-w-md rounded-3xl border border-border/60 bg-background/95 p-8 shadow-2xl shadow-black/5">
          <p className="text-sm font-semibold uppercase tracking-[0.28em] text-muted-foreground">
            Authentification
          </p>
          <h1 className="mt-3 text-3xl font-semibold tracking-tight">
            Impossible de terminer la connexion
          </h1>
          <p className="mt-4 text-sm leading-6 text-muted-foreground">{error}</p>
          <div className="mt-6 flex items-center gap-3">
            <a
              href="/"
              className="inline-flex h-10 items-center rounded-full bg-foreground px-4 text-sm font-medium text-background transition hover:opacity-90"
            >
              Retourner à Drive
            </a>
            <a
              href="mailto:support@nvbes.fr?subject=Erreur%20Drive%20callback"
              className="inline-flex h-10 items-center rounded-full border border-border bg-background px-4 text-sm font-medium text-foreground transition hover:bg-muted"
            >
              Signaler
            </a>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(15,23,42,0.08),_transparent_35%),linear-gradient(180deg,_#fbfcfe_0%,_#f3f6fb_100%)] px-4">
      <div className="w-full max-w-md rounded-3xl border border-border/60 bg-background/95 p-8 shadow-2xl shadow-black/5">
        <p className="text-sm font-semibold uppercase tracking-[0.28em] text-muted-foreground">
          Authentification
        </p>
        <h1 className="mt-3 text-3xl font-semibold tracking-tight">Connexion en cours</h1>
        <p className="mt-4 text-sm leading-6 text-muted-foreground">
          Nous validons votre session Identity et préparons votre espace Drive.
        </p>
      </div>
    </div>
  );
}
