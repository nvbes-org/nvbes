import type { AccountEntry } from '@nvbes/identity-client';
import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';

export function LoginPageChooser({
  accounts,
  onAccountSelect,
  onUseAnotherAccount,
}: {
  accounts: AccountEntry[];
  onAccountSelect: (authuser: string) => void;
  onUseAnotherAccount: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex max-h-[280px] flex-col gap-2 overflow-y-auto pr-1">
        {accounts.map((account) => {
          const initials = (account.user.display_name || account.user.email || '?')
            .slice(0, 1)
            .toUpperCase();
          return (
            <button
              key={account.authuser}
              type="button"
              onClick={() => onAccountSelect(account.authuser)}
              className="flex w-full items-center gap-3 rounded-xl border border-border/60 bg-card p-3 text-left transition hover:border-primary/20 hover:bg-muted"
            >
              <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">
                {initials}
              </div>
              <div className="flex min-w-0 flex-col">
                <span className="truncate text-sm font-medium">{account.user.display_name}</span>
                <span className="truncate text-xs text-muted-foreground">{account.user.email}</span>
              </div>
            </button>
          );
        })}
      </div>

      <Separator />

      <Button
        type="button"
        variant="outline"
        onClick={onUseAnotherAccount}
        className="flex w-full items-center justify-center gap-2"
      >
        <Plus className="size-4" />
        Se connecter à un autre compte
      </Button>
    </div>
  );
}
