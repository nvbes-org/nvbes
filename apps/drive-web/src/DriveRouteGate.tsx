import { AuthErrorBoundary, runClientEffect } from '@nvbes/web-runtime';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Suspense, useEffect, useState } from 'react';
import { DriveShell } from './DriveShell';
import { driveMeQueryOptions } from './drive.queries';
import { getAccessToken, subscribeToSessionChanges } from './drive.session';
import { startDriveLoginWorkflow } from './drive.workflow';
import { syncTrackingConsent } from './tracking-consent';

export function DriveRouteGate() {
  const [accessToken, setAccessTokenState] = useState(getAccessToken());

  useEffect(() => {
    return subscribeToSessionChanges(() => {
      setAccessTokenState(getAccessToken());
    });
  }, []);

  useEffect(() => {
    if (!accessToken) {
      void runClientEffect(startDriveLoginWorkflow());
    }
  }, [accessToken]);

  useEffect(() => {
    if (!accessToken) {
      return;
    }

    void syncTrackingConsent();
  }, [accessToken]);

  if (!accessToken) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <div className="text-center">
          <div className="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
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
            <div className="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
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
      void runClientEffect(startDriveLoginWorkflow());
    }
  }, [data]);

  if (!data) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <div className="text-center">
          <div className="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent mx-auto" />
          <p className="mt-4 text-sm text-muted-foreground">Redirection vers nvbes Identity...</p>
        </div>
      </div>
    );
  }

  return <DriveShell accessToken={accessToken} me={data} />;
}
