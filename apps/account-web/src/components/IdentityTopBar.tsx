import { Link, useRouterState } from '@tanstack/react-router';
import { CircleOffIcon } from 'lucide-react';

import { AccountChooser } from '@/components/AccountChooser';
import { useAccountContext } from '@/hooks/useAccountContext';

function IdentityTopBarAccountChooser() {
  const { accounts, loading } = useAccountContext();

  return (
    <AccountChooser
      accounts={accounts}
      loading={loading}
      density="compact"
      className="w-[min(20rem,calc(100vw-7rem))] bg-transparent p-0"
    />
  );
}

export function IdentityTopBar() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const showAccountChooser = pathname.startsWith('/account');

  return (
    <header className="sticky top-0 z-40 flex h-14 shrink-0 items-center border-b border-border bg-background/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/80">
      <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-4">
        <Link to="/login" className="flex min-w-0 items-center gap-2.5">
          <span className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/15 text-primary">
            <CircleOffIcon aria-hidden="true" className="size-4 stroke-[2.5]" />
          </span>
          <span className="truncate text-sm font-semibold tracking-tight text-foreground">
            nvbes Identity
          </span>
        </Link>

        {showAccountChooser ? <IdentityTopBarAccountChooser /> : null}
      </div>
    </header>
  );
}
