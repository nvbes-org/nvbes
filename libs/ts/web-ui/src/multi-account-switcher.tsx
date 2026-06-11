import { Check, ChevronDown, Trash2, UserRound } from 'lucide-react';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

import { cn } from './lib/classnames';
import {
  SwitcherAvatar,
  SwitcherBadge,
  SwitcherButton,
  SwitcherCard,
  SwitcherSeparator,
  SwitcherSpinner,
} from './multi-account-switcher.parts';
import {
  findActiveAccount,
  getSwitcherMenuStyle,
  initialsForDisplayName,
  type SharedAccountOption,
} from './multi-account-switcher.utils';

export interface MultiAccountSwitcherProps {
  accounts: SharedAccountOption[];
  loading: boolean;
  onSelectAccount: (accountId: string) => void;
  onAddAccount: () => void;
  switchingAccountId?: string | null;
  onRemoveAccount?: (accountId: string) => void;
  addAccountLabel?: string;
  menuTitle?: string;
  menuDescription?: string;
  loadingLabel?: string;
  emptyLabel?: string;
  className?: string;
  footerActions?: ReactNode;
}

export function MultiAccountSwitcher({
  accounts,
  loading,
  onSelectAccount,
  onAddAccount,
  switchingAccountId = null,
  onRemoveAccount,
  addAccountLabel = 'Se connecter à un autre compte',
  menuTitle = 'Comptes connectés',
  menuDescription = 'Basculer entre les sessions ouvertes.',
  loadingLabel = 'Chargement des comptes...',
  emptyLabel = 'Aucun compte connecté.',
  className,
  footerActions,
}: MultiAccountSwitcherProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [style, setStyle] = useState<Record<string, string>>({});
  const currentAccount = findActiveAccount(accounts);
  const currentAvatarFallback =
    currentAccount?.avatarFallback ?? initialsForDisplayName(currentAccount?.displayName ?? '');

  const updatePosition = useCallback(() => {
    if (!open || !rootRef.current) {
      return;
    }

    const rect = rootRef.current.getBoundingClientRect();
    setStyle(getSwitcherMenuStyle(rect));
  }, [open]);

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    updatePosition();
    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [open, updatePosition]);

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;

      if (!(target instanceof Node)) {
        return;
      }

      if (rootRef.current?.contains(target)) {
        return;
      }

      if ((target as Element).closest('[data-switcher-portal]')) {
        return;
      }

      setOpen(false);
    };

    document.addEventListener('pointerdown', handlePointerDown);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown);
    };
  }, [open]);

  return (
    <div ref={rootRef} className={className}>
      <SwitcherButton
        type="button"
        variant="ghost"
        className="h-auto w-full justify-between px-1 py-0.5 text-left"
        onClick={() => setOpen((value) => !value)}
      >
        <div className="flex min-w-0 items-center gap-2.5">
          <SwitcherAvatar className="size-10 bg-primary/10 text-primary">
            <span className="text-sm font-medium">
              {currentAvatarFallback || <UserRound className="size-4" />}
            </span>
          </SwitcherAvatar>
          <div className="flex min-w-0 flex-col">
            <span className="font-heading truncate text-sm font-medium leading-tight">
              {currentAccount?.displayName ?? 'Compte'}
            </span>
            <span className="truncate text-[0.68rem] text-muted-foreground leading-tight">
              {currentAccount?.email ?? 'Aucun compte actif'}
            </span>
          </div>
        </div>
        <ChevronDown
          className={cn(
            'size-4 shrink-0 text-muted-foreground transition-transform',
            open && 'rotate-180',
          )}
        />
      </SwitcherButton>

      {open
        ? createPortal(
            <SwitcherCard
              data-switcher-portal="true"
              className="fixed z-[9999] max-w-[calc(100vw-2rem)] p-4 shadow-lg shadow-black/10"
              style={style}
            >
              <div className="flex items-center justify-between gap-3 pb-3">
                <div className="min-w-0">
                  <p className="font-heading text-sm font-medium">{menuTitle}</p>
                  <p className="mt-0.5 text-xs text-muted-foreground">{menuDescription}</p>
                </div>
              </div>
              <SwitcherSeparator />

              <div className="mt-3 max-h-[24rem] overflow-auto">
                {loading ? (
                  <div className="flex items-center gap-2.5 px-2 py-2 text-sm text-muted-foreground">
                    <SwitcherSpinner />
                    {loadingLabel}
                  </div>
                ) : accounts.length === 0 ? (
                  <div className="px-2 py-2 text-sm text-muted-foreground">{emptyLabel}</div>
                ) : (
                  <div className="space-y-2">
                    {accounts.map((account) => {
                      const switching = switchingAccountId === account.id;

                      return (
                        <div key={account.id} className="flex items-center gap-2">
                          <SwitcherButton
                            type="button"
                            variant="ghost"
                            className={cn(
                              'h-auto min-w-0 flex-1 justify-between gap-3 px-3 py-2.5 text-left',
                              account.isActive && 'bg-primary/10 text-primary',
                            )}
                            onClick={() => onSelectAccount(account.id)}
                            disabled={switching}
                          >
                            <div className="flex min-w-0 items-center gap-3">
                              <SwitcherAvatar className="size-9">
                                <span className="text-sm">
                                  {account.avatarFallback ||
                                    initialsForDisplayName(account.displayName) || (
                                      <UserRound className="size-4" />
                                    )}
                                </span>
                              </SwitcherAvatar>
                              <div className="flex min-w-0 flex-col">
                                <span className="truncate font-medium">{account.displayName}</span>
                                <span className="truncate text-xs text-muted-foreground">
                                  {account.email}
                                </span>
                              </div>
                            </div>
                            <span className="flex shrink-0 items-center gap-2">
                              {switching ? (
                                <SwitcherSpinner className="size-3.5" />
                              ) : account.isActive ? (
                                <Check className="size-3.5" />
                              ) : (
                                <SwitcherBadge>Switch</SwitcherBadge>
                              )}
                            </span>
                          </SwitcherButton>

                          {onRemoveAccount && !account.isActive ? (
                            <SwitcherButton
                              type="button"
                              variant="ghost"
                              size="icon-sm"
                              className="shrink-0 hover:text-destructive"
                              aria-label={`Retirer ${account.displayName}`}
                              onClick={() => onRemoveAccount(account.id)}
                            >
                              <Trash2 />
                            </SwitcherButton>
                          ) : null}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="mt-4 pt-3">
                <SwitcherSeparator className="mb-3" />
                <SwitcherButton type="button" className="w-full" onClick={onAddAccount}>
                  {addAccountLabel}
                </SwitcherButton>
              </div>

              {footerActions ? (
                <>
                  <SwitcherSeparator className="mt-3" />
                  <div className="pt-3">{footerActions}</div>
                </>
              ) : null}
            </SwitcherCard>,
            document.body,
          )
        : null}
    </div>
  );
}
