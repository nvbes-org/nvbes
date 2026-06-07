import { Check, ChevronDown, Loader2, UserRound } from 'lucide-react';

import { Button } from '@/components/ui/button';
import type { AccountEntry } from '@/lib/account-context';
import { cn } from '@/lib/classnames';

export interface AccountChooserProps {
  accounts: AccountEntry[];
  loading: boolean;
}

export function AccountChooserTrigger({
  currentAccount,
  open,
  onToggle,
}: {
  currentAccount: AccountEntry | null;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      className="flex w-full items-center justify-between gap-3 rounded-xl border border-border/70 bg-background/80 px-3 py-2 text-left shadow-sm transition hover:border-primary/30 hover:bg-background"
      onClick={onToggle}
    >
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
          <UserRound className="size-4" />
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="font-heading truncate text-sm font-medium">
            {currentAccount?.user.display_name ?? 'Compte'}
          </span>
          <span className="truncate text-xs text-muted-foreground">
            {currentAccount?.user.email ?? 'Aucun compte actif'}
          </span>
        </div>
      </div>
      <ChevronDown className={cn('size-4 shrink-0 transition-transform', open && 'rotate-180')} />
    </button>
  );
}

export function AccountChooserMenu({
  accounts,
  loading,
  switchingTo,
  onSwitch,
}: {
  accounts: AccountEntry[];
  loading: boolean;
  switchingTo: string | null;
  onSwitch: (authuser: string) => void;
}) {
  return (
    <div className="absolute left-4 right-4 top-[calc(100%-0.25rem)] min-w-[22rem] rounded-xl bg-card p-4 shadow-lg shadow-black/10 ring-1 ring-foreground/10 md:min-w-[24rem] lg:min-w-[26rem]">
      <div className="flex items-center justify-between gap-3 border-b border-border pb-3">
        <div className="min-w-0">
          <p className="font-heading text-sm font-medium">Comptes connectés</p>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Basculer entre les sessions ouvertes.
          </p>
        </div>
      </div>

      <div className="mt-3 max-h-[24rem] overflow-auto">
        {loading ? (
          <div className="flex items-center gap-2.5 px-2 py-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Chargement des comptes...
          </div>
        ) : accounts.length === 0 ? (
          <div className="px-2 py-2 text-sm text-muted-foreground">Aucun compte connecté.</div>
        ) : (
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
                    active ? 'bg-primary/10 text-primary' : 'hover:bg-muted hover:text-foreground',
                  )}
                  onClick={() => onSwitch(account.authuser)}
                  disabled={switching}
                >
                  <div className="flex min-w-0 items-center gap-3">
                    <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted">
                      <UserRound className="size-4 text-muted-foreground" />
                    </div>
                    <div className="flex min-w-0 flex-col">
                      <span className="truncate font-medium">{account.user.display_name}</span>
                      <span className="truncate text-xs text-muted-foreground">
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
  );
}
