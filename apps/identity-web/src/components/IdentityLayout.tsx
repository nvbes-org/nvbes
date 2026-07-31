import { Outlet, useNavigate } from '@tanstack/react-router';
import { useEffect, useTransition } from 'react';

import { identityAuthenticationDisposition } from '@/components/identity-layout.authentication';
import { IdentitySidebar } from '@/components/IdentitySidebar';
import { IdentityTopBar } from '@/components/IdentityTopBar';
import { SkeletonGroup } from '@/components/SkeletonGroup';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useIdentityContext } from '@/hooks/useIdentityContext';

function LayoutSkeleton() {
  return (
    <div className="flex h-[calc(100vh-3.5rem)]">
      <aside className="hidden w-64 shrink-0 md:flex md:flex-col">
        <div className="flex flex-1 flex-col gap-4 px-3">
          <SkeletonGroup count={5} itemClassName="h-9 w-full rounded-lg" />
        </div>
      </aside>
      <main className="flex flex-1 flex-col gap-10 overflow-auto px-4 md:px-8">
        <div className="mx-auto flex w-full max-w-2xl flex-col gap-4">
          <Skeleton className="h-12 w-full max-w-lg" />
          <Skeleton className="h-6 w-full max-w-md" />
          <SkeletonGroup count={4} itemClassName="h-20 w-full" />
        </div>
      </main>
    </div>
  );
}

export default function IdentityLayout() {
  const navigate = useNavigate();
  const [isPending, startTransition] = useTransition();
  const { me, loading, error, retry, retrying } = useIdentityContext();
  const authenticationDisposition = identityAuthenticationDisposition(error);
  const reauthenticationRequired = authenticationDisposition === 'reauthenticate';
  const blockingError = error && (reauthenticationRequired || !me) ? error : null;

  useEffect(() => {
    if (!loading && authenticationDisposition === 'redirect-login') {
      void navigate({
        to: '/login',
        search: { return_to: window.location.pathname },
        replace: true,
      });
    }
  }, [authenticationDisposition, loading, navigate]);

  return (
    <div className="min-h-screen bg-background">
      <IdentityTopBar />
      {loading ? (
        <LayoutSkeleton />
      ) : blockingError ? (
        <div className="flex h-[calc(100vh-3.5rem)] items-center justify-center px-4">
          <div className="flex w-full max-w-md flex-col gap-3 text-center">
            <h1 className="text-3xl text-muted-foreground">
              {reauthenticationRequired
                ? 'Votre session a expiré.'
                : 'Impossible de charger votre identité.'}
            </h1>
            <Button
              onClick={() => {
                if (!reauthenticationRequired) {
                  retry();
                  return;
                }
                startTransition(() => {
                  void navigate({ to: '/login' });
                });
              }}
              disabled={isPending || retrying}
            >
              {reauthenticationRequired
                ? isPending
                  ? 'Redirection…'
                  : 'Se reconnecter'
                : retrying
                  ? 'Nouvelle tentative…'
                  : 'Réessayer'}
            </Button>
          </div>
        </div>
      ) : !me ? (
        <LayoutSkeleton />
      ) : (
        <div className="flex h-[calc(100vh-3.5rem)]">
          <aside className="hidden w-64 shrink-0 md:flex md:flex-col">
            <IdentitySidebar />
          </aside>
          <main className="flex-1 overflow-auto">
            <div className="mx-auto w-full max-w-2xl px-4 md:px-8">
              <Outlet />
            </div>
          </main>
        </div>
      )}
    </div>
  );
}
