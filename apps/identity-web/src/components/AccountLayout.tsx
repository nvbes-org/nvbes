import { preventAutoSignIn } from '@nvbes/identity-sdk-web';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Outlet, useNavigate } from '@tanstack/react-router';
import { Menu } from 'lucide-react';
import { useTransition } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { AccountSidebar } from '@/components/AccountSidebar';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';
import { logoutIdentitySessionMutationFn } from '@/identity.auth.queries';

function LayoutSkeleton() {
  return (
    <div className="flex h-screen">
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
  const queryClient = useQueryClient();
  const [isPending, startTransition] = useTransition();
  const { me, accounts, workspaces, loading } = useAccountContext();
  const logoutMutation = useMutation({ mutationFn: logoutIdentitySessionMutationFn });
  const currentWorkspaceRole =
    workspaces.find((workspace) => workspace.id === me?.current_workspace_id)?.role ?? null;

  const handleLogout = async () => {
    try {
      await logoutMutation.mutateAsync(undefined);
    } finally {
      queryClient.removeQueries({ queryKey: accountQueryKeys.all });
      preventAutoSignIn().catch(() => {});
      startTransition(() => {
        void navigate({ to: '/login', replace: true });
      });
    }
  };

  if (loading) return <LayoutSkeleton />;
  if (!me) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="flex flex-col gap-3 text-center">
          <p className="text-sm text-muted-foreground">Session expiree</p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => void navigate({ to: '/login' })}
            disabled={isPending}
          >
            {isPending ? 'Redirection...' : 'Se reconnecter'}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-background">
      {/* Desktop sidebar */}
      <aside className="hidden w-60 shrink-0 border-r border-border bg-card md:flex md:flex-col">
        <AccountSidebar
          accounts={accounts}
          loadingAccounts={loading}
          currentWorkspaceRole={currentWorkspaceRole}
          onLogout={handleLogout}
        />
      </aside>

      {/* Mobile top bar + Sheet */}
      <div className="flex flex-1 flex-col min-w-0">
        <div className="flex items-center gap-3 border-b border-border bg-card px-4 h-12 md:hidden shrink-0">
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon-sm">
                <Menu className="size-4" />
                <span className="sr-only">Menu</span>
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-60 p-0">
              <SheetTitle className="sr-only">Navigation</SheetTitle>
              <AccountSidebar
                accounts={accounts}
                loadingAccounts={loading}
                currentWorkspaceRole={currentWorkspaceRole}
                onLogout={handleLogout}
              />
            </SheetContent>
          </Sheet>
          <span className="text-sm font-heading font-medium truncate">Mon compte</span>
        </div>

        <main className="flex-1 overflow-auto">
          <div className="mx-auto w-full max-w-2xl px-4 py-8 md:px-8 md:py-12">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
