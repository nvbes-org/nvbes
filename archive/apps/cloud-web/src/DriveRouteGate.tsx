import { AuthErrorBoundary } from '@nvbes/web-runtime';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Suspense, useEffect, useState } from 'react';
import { Spinner } from '@/components/ui/spinner';
import { startDriveLogin } from './drive.auth.functions';
import { DriveShell } from './DriveShell';
import { driveMeQueryOptions } from './drive.queries';
import { getAccessToken, getValidAccessToken, subscribeToSessionChanges } from './drive.session';
import { syncTrackingConsent } from './tracking-consent';

export function DriveRouteGate() {
  const [accessToken, setAccessTokenState] = useState(getAccessToken());
  const [checkingRefresh, setCheckingRefresh] = useState(false);

  useEffect(() => {
    return subscribeToSessionChanges(() => {
      setAccessTokenState(getAccessToken());
    });
  }, []);

  useEffect(() => {
    if (!accessToken) {
      let cancelled = false;
      setCheckingRefresh(true);
      void getValidAccessToken()
        .then((token) => {
          if (cancelled) {
            return;
          }
          if (token) {
            setAccessTokenState(token);
            return;
          }
          void startDriveLogin();
        })
        .finally(() => {
          if (!cancelled) {
            setCheckingRefresh(false);
          }
        });
      return () => {
        cancelled = true;
      };
    }
    setCheckingRefresh(false);
    return undefined;
  }, [accessToken]);

  useEffect(() => {
    if (!accessToken) {
      return;
    }

    void syncTrackingConsent().catch(() => {
      // Tracking consent sync is best-effort and must not break Drive bootstrap.
    });
  }, [accessToken]);

  if (!accessToken || checkingRefresh) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <div className="text-center">
          <Spinner className="mx-auto size-8 text-primary" />
          <p className="mt-4 text-sm text-muted-foreground">Redirection vers nvbes Identity...</p>
        </div>
      </div>
    );
  }

  return (
    <Suspense
      fallback={
        <div className="flex min-h-svh items-center justify-center bg-background">
          <div className="text-center">
            <Spinner className="mx-auto size-8 text-primary" />
            <p className="mt-4 text-sm text-muted-foreground">Chargement...</p>
          </div>
        </div>
      }
    >
      <AuthErrorBoundary>
        <DriveSessionGate accessToken={accessToken} />
      </AuthErrorBoundary>
    </Suspense>
  );
}

function DriveSessionGate({ accessToken }: { accessToken: string }) {
  const { data } = useSuspenseQuery(driveMeQueryOptions(accessToken));

  useEffect(() => {
    if (!data) {
      void startDriveLogin();
    }
  }, [data]);

  if (!data) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <div className="text-center">
          <Spinner className="mx-auto size-8 text-primary" />
          <p className="mt-4 text-sm text-muted-foreground">Redirection vers nvbes Identity...</p>
        </div>
      </div>
    );
  }

  return <DriveShell accessToken={accessToken} me={data} />;
}
