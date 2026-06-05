import { useLocation } from '@tanstack/react-router';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Check, ChevronDown, Loader2, UserRound } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { Button } from '@/components/ui/button';
import type { AccountEntry } from '@/lib/account-context';
import { cn } from '@/lib/utils';

interface AccountChooserProps {
  accounts: AccountEntry[];
  loading: boolean;
}

export function AccountChooser({ accounts, loading }: AccountChooserProps) {
  const location = useLocation();
  const rootRef = useRef<HTMLDivElement | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [switchingTo, setSwitchingTo] = useState<string | null>(null);

  const currentAccount = useMemo(
    () => accounts.find((account) => account.session.current) ?? accounts[0] ?? null,
    [accounts],
  );

  const handleSwitch = (authuser: string) => {
    setSwitchingTo(authuser);
    const url = new URL(window.location.href);
    url.searchParams.set('authuser', authuser);
    url.searchParams.set('from', location.pathname);
    window.location.assign(url.toString());
  };

  const rowCount = accounts.length > 0 ? accounts.length : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: () => 56,
    overscan: 6,
    enabled: open && !loading && accounts.length > 6,
  });

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;

      if (!(target instanceof Node)) {
        return;
      }

      if (!rootRef.current?.contains(target)) {
        setOpen(false);
      }
    };

    document.addEventListener('pointerdown', handlePointerDown);

    return () => {
      document.removeEventListener('pointerdown', handlePointerDown);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative z-20 bg-card px-4 py-4">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 rounded-xl border border-border/70 bg-background/80 px-3 py-2 text-left shadow-sm transition hover:border-primary/30 hover:bg-background"
        onClick={() => setOpen((value) => !value)}
      >
        <div className="flex items-center gap-3 min-w-0">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
            <UserRound className="size-4" />
          </div>
          <div className="flex flex-col min-w-0">
            <span className="text-sm font-heading font-medium truncate">
              {currentAccount?.user.display_name ?? 'Compte'}
            </span>
            <span className="text-xs text-muted-foreground truncate">
              {currentAccount?.user.email ?? 'Aucun compte actif'}
            </span>
          </div>
        </div>
        <ChevronDown className={cn('size-4 shrink-0 transition-transform', open && 'rotate-180')} />
      </button>

      {open && (
        <div className="absolute left-4 right-4 top-[calc(100%-0.25rem)] min-w-[22rem] rounded-xl bg-card p-4 shadow-lg shadow-black/10 ring-1 ring-foreground/10 md:min-w-[24rem] lg:min-w-[26rem]">
          <div className="flex items-center justify-between gap-3 border-b border-border pb-3">
            <div className="min-w-0">
              <p className="font-heading text-sm font-medium">Comptes connectés</p>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Basculer entre les sessions ouvertes.
              </p>
            </div>
          </div>

          <div ref={listRef} className="mt-3 max-h-[24rem] overflow-auto">
            {loading ? (
              <div className="flex items-center gap-2.5 px-2 py-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                Chargement des comptes...
              </div>
            ) : accounts.length === 0 ? (
              <div className="px-2 py-2 text-sm text-muted-foreground">Aucun compte connecté.</div>
            ) : accounts.length <= 6 ? (
              <div className="space-y-2">
                {accounts.map((account) => {
                  const active = account.session.current;
                  const switching = switchingTo === account.authuser;

                  return (
                    <button
                      key={account.authuser}
                      type="button"
                      className={cn(
                        'flex w-full items-center justify-between gap-3 rounded-lg px-3 py-2.5 text-left text-sm transition',
                        active
                          ? 'bg-primary/10 text-primary'
                          : 'hover:bg-muted hover:text-foreground',
                      )}
                      onClick={() => handleSwitch(account.authuser)}
                      disabled={switching}
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted">
                          <UserRound className="size-4 text-muted-foreground" />
                        </div>
                        <div className="flex flex-col min-w-0">
                          <span className="font-medium truncate">{account.user.display_name}</span>
                          <span className="text-xs text-muted-foreground truncate">
                            {account.user.email}
                          </span>
                        </div>
                      </div>
                      <span className="flex shrink-0 items-center gap-2 text-xs">
                        {switching ? (
                          <Loader2 className="size-3.5 animate-spin" />
                        ) : active ? (
                          <Check className="size-3.5" />
                        ) : (
                          <Button variant="outline" size="sm" className="h-6 px-2.5">
                            Switch
                          </Button>
                        )}
                      </span>
                    </button>
                  );
                })}
              </div>
            ) : (
              <div
                style={{
                  height: `${virtualizer.getTotalSize()}px`,
                  position: 'relative',
                }}
              >
                {virtualizer.getVirtualItems().map((virtualItem) => {
                  const account = accounts[virtualItem.index];
                  const active = account.session.current;
                  const switching = switchingTo === account.authuser;

                  return (
                    <button
                      key={account.authuser}
                      ref={virtualizer.measureElement}
                      data-index={virtualItem.index}
                      type="button"
                      className={cn(
                        'absolute left-0 right-0 flex w-full items-center justify-between gap-3 rounded-lg px-3 py-2.5 text-left text-sm transition',
                        active
                          ? 'bg-primary/10 text-primary'
                          : 'hover:bg-muted hover:text-foreground',
                      )}
                      onClick={() => handleSwitch(account.authuser)}
                      disabled={switching}
                      style={{ transform: `translateY(${virtualItem.start}px)` }}
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted">
                          <UserRound className="size-4 text-muted-foreground" />
                        </div>
                        <div className="flex flex-col min-w-0">
                          <span className="font-medium truncate">{account.user.display_name}</span>
                          <span className="text-xs text-muted-foreground truncate">
                            {account.user.email}
                          </span>
                        </div>
                      </div>
                      <span className="flex shrink-0 items-center gap-2 text-xs">
                        {switching ? (
                          <Loader2 className="size-3.5 animate-spin" />
                        ) : active ? (
                          <Check className="size-3.5" />
                        ) : (
                          <Button variant="outline" size="sm" className="h-6 px-2.5">
                            Switch
                          </Button>
                        )}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
