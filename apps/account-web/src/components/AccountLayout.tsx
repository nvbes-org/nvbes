import { Outlet, useNavigate } from '@tanstack/react-router';
import { isSessionStaleError } from '@nvbes/web-runtime';
import { useTransition } from 'react';
import { AccountSidebar } from '@/components/AccountSidebar';
import { IdentityTopBar } from '@/components/IdentityTopBar';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';

function LayoutSkeleton() {
  return (
    <div className="flex h-[calc(100vh-3.5rem)]">
      <aside className="hidden w-60 shrink-0 border-r border-border bg-card md:flex flex-col">
        <div className="flex items-center gap-3 px-4 pt-5 pb-4">
          <Skeleton className="size-10 rounded-full" />
          <div className="flex flex-col gap-1.5">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-3 w-32" />
          </div>
        </div>
        <Skeleton className="h-px w-full" />
        <div className="flex-1 px-3 py-4 flex flex-col gap-4">
          {Array.from({ length: 12 }).map((_, i) => (
            <Skeleton key={i} className="h-6 w-full rounded-lg" />
          ))}
        </div>
      </aside>
      <main className="flex-1 flex items-center justify-center">
        <Skeleton className="h-8 w-24" />
      </main>
    </div>
  );
}

export default function AccountLayout() {
  const navigate = useNavigate();
  const [isPending, startTransition] = useTransition();
  const { me, loading, error, retry, retrying } = useAccountContext();
  const reauthenticationRequired = isSessionStaleError(error);
  const blockingError = error && (reauthenticationRequired || !me) ? error : null;

  return (
    <div className="min-h-screen bg-background">
      <IdentityTopBar />

      {loading ? (
        <LayoutSkeleton />
      ) : blockingError ? (
        <div className="flex h-[calc(100vh-3.5rem)] items-center justify-center">
          <div className="flex flex-col gap-3 text-center">
            <p className="text-sm text-muted-foreground">
              {reauthenticationRequired
                ? 'Votre session a expiré.'
                : 'Impossible de charger votre compte.'}
            </p>
            <Button
              variant="outline"
              size="sm"
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
                  ? 'Redirection...'
                  : 'Se reconnecter'
                : retrying
                  ? 'Nouvelle tentative...'
                  : 'Réessayer'}
            </Button>
          </div>
        </div>
      ) : !me ? (
        <LayoutSkeleton />
      ) : (
        <div className="flex h-[calc(100vh-3.5rem)] bg-background">
          {/* Desktop sidebar */}
          <aside className="hidden w-60 shrink-0 bg-card md:flex md:flex-col">
            <AccountSidebar />
          </aside>

          {/* Mobile top bar + Sheet */}
          <div className="flex flex-1 flex-col min-w-0">
            <main className="flex-1 overflow-auto">
              <div className="mx-auto w-full max-w-2xl px-4 py-8 md:px-8 md:py-12">
                <Outlet />
              </div>
            </main>
          </div>
        </div>
      )}
    </div>
  );
}
