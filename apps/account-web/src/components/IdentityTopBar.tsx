import { Link, useRouterState } from '@tanstack/react-router';

import { AccountChooser } from '@/components/AccountChooser';
import { useAccountContext } from '@/hooks/useAccountContext';
import { AccountSidebar } from '@/components/AccountSidebar';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';
import { Menu } from 'lucide-react';

function IdentityTopBarAccountChooser() {
  const { accounts, loading } = useAccountContext();

  return (
    <AccountChooser
      accounts={accounts}
      loading={loading}
      density="compact"
      className="w-fit bg-transparent p-0 md:w-[min(20rem,calc(100vw-7rem))]"
    />
  );
}

export function IdentityTopBar() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const showAccountChooser = pathname.startsWith('/account');

  return (
    <header className="sticky top-0 z-40 flex h-14 shrink-0 items-center bg-background/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/80">
      <div className="mx-auto flex w-full items-center justify-between gap-4">
        <div className="flex flex-row">
          <div className="flex items-center gap-3 pr-1 md:hidden shrink-0">
            <div className="-mb-1.25">
              <Sheet>
                <SheetTrigger asChild>
                  <Button variant="ghost" size="icon-sm">
                    <Menu className="size-4" />
                    <span className="sr-only">Menu</span>
                  </Button>
                </SheetTrigger>
                <SheetContent side="left" className="w-60 p-0">
                  <AccountSidebar />
                </SheetContent>
              </Sheet>
            </div>
          </div>
          <Link to="/login" className="flex min-w-0 items-center gap-2.5">
            <span className="truncate text-2xl font-light tracking-tight text-foreground">
              <span className="font-semibold">nvbes</span> Compte
            </span>
          </Link>
        </div>

        {showAccountChooser ? <IdentityTopBarAccountChooser /> : null}
      </div>
    </header>
  );
}
