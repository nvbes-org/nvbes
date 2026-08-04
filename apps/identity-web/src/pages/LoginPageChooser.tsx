import type { AccountEntry } from '@nvbes/identity-client';
import { LogOut, Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';

export function LoginPageChooser({
  accounts,
  onAccountSelect,
  onDisconnectAccount,
  onDisconnectAllAccounts,
  onUseAnotherAccount,
}: {
  accounts: AccountEntry[];
  onAccountSelect: (authuser: string) => void;
  onDisconnectAccount: (authuser: string) => void | Promise<void>;
  onDisconnectAllAccounts: () => void | Promise<void>;
  onUseAnotherAccount: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex max-h-[280px] flex-col gap-2 overflow-y-auto pr-1">
        {accounts.map((account) => {
          const displayName = account.user.display_name.trim() || 'User';
          const initials = displayName.slice(0, 1).toUpperCase();
          const expired = account.status === 'expired';
          return (
            <div key={account.authuser} className="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => onAccountSelect(account.authuser)}
                className="h-auto min-w-0 flex-1 justify-start gap-3 p-3 text-left"
              >
                <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">
                  {initials}
                </div>
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-sm font-medium">{displayName}</span>
                  <span className="truncate text-xs text-muted-foreground">
                    {account.user.email}
                  </span>
                  {expired ? (
                    <span className="mt-1 text-xs font-medium text-destructive">
                      {account.message ?? 'Session expirée, veuillez vous reconnecter.'}
                    </span>
                  ) : null}
                </div>
                {expired ? (
                  <span className="shrink-0 text-xs font-medium text-primary">Se reconnecter</span>
                ) : null}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="shrink-0"
                onClick={(event) => {
                  event.stopPropagation();
                  void onDisconnectAccount(account.authuser);
                }}
                aria-label={`Deconnecter ${account.user.email}`}
              >
                <LogOut />
              </Button>
            </div>
          );
        })}
      </div>

      <Separator />

      <div className="flex flex-col gap-2">
        <Button
          type="button"
          variant="outline"
          onClick={onUseAnotherAccount}
          className="flex w-full items-center justify-center gap-2"
        >
          <Plus data-icon="inline-start" />
          Se connecter à un autre compte
        </Button>

        <Button type="button" variant="ghost" onClick={onDisconnectAllAccounts} className="w-full">
          Deconnecter tous les comptes
        </Button>
      </div>
    </div>
  );
}
