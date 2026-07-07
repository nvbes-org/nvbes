import { clientErrorMessage } from '@nvbes/web-runtime';
import { useEffect, useState } from 'react';
import { completeDeveloperCallback } from '../developer.auth';

let callbackExchangeStarted = false;

export function DeveloperCallbackPage() {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (callbackExchangeStarted) {
      return;
    }
    callbackExchangeStarted = true;

    let cancelled = false;
    const searchParams = new URLSearchParams(window.location.search);

    async function run(): Promise<void> {
      try {
        const returnTo = await completeDeveloperCallback(searchParams);
        window.location.assign(returnTo);
      } catch (err) {
        if (!cancelled) {
          setError(clientErrorMessage(err, 'OAuth callback failed.'));
        }
      }
    }

    void run();

    return () => {
      cancelled = true;
      window.setTimeout(() => {
        callbackExchangeStarted = false;
      }, 0);
    };
  }, []);

  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
      <div className="w-full max-w-md rounded-md border border-border bg-card p-6">
        <p className="text-xs font-medium uppercase text-muted-foreground">Console</p>
        <h1 className="mt-2 text-lg font-semibold">
          {error ? 'Connection failed' : 'Connection in progress'}
        </h1>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">
          {error || 'We are validating your Identity authorization.'}
        </p>
        {error ? (
          <a
            href="/portal/apps"
            className="mt-5 inline-flex rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
          >
            Retry
          </a>
        ) : null}
      </div>
    </main>
  );
}
