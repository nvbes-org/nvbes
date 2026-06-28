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
  density?: 'default' | 'compact';
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
  density = 'default',
  footerActions,
}: MultiAccountSwitcherProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [style, setStyle] = useState<Record<string, string>>({});
  const currentAccount = findActiveAccount(accounts);
  const currentAvatarFallback =
    currentAccount?.avatarFallback ?? initialsForDisplayName(currentAccount?.displayName ?? '');
  const compact = density === 'compact';

  const updatePosition = useCallback(() => {
    if (!open || !rootRef.current) {
      return;
    }

    const rect = rootRef.current.getBoundingClientRect();
    setStyle(getSwitcherMenuStyle(rect, compact ? { minWidth: 320, offset: 6 } : undefined));
  }, [compact, open]);

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
        className={cn(
          'w-full justify-between text-left',
          compact ? 'h-10 rounded-xl px-2 py-1' : 'h-auto px-1 py-0.5',
        )}
        onClick={() => setOpen((value) => !value)}
      >
        <div className={cn('flex min-w-0 items-center', compact ? 'gap-2' : 'gap-2.5')}>
          <SwitcherAvatar
            className={cn('bg-primary/10 text-primary', compact ? 'size-8' : 'size-10')}
          >
            <span className={cn('font-medium', compact ? 'text-xs' : 'text-sm')}>
              {currentAvatarFallback || <UserRound className="size-4" />}
            </span>
          </SwitcherAvatar>
          <div className="flex min-w-0 flex-col">
            <span className="font-heading truncate text-sm font-medium leading-tight">
              {currentAccount?.displayName ?? 'Compte'}
            </span>
            <span className="truncate text-[0.68rem] leading-tight text-muted-foreground">
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
              className={cn(
                'fixed z-[9999] max-w-[calc(100vw-1rem)] shadow-lg shadow-black/10',
                compact ? 'p-3' : 'p-4',
              )}
              style={style}
            >
              <div
                className={cn('flex items-center justify-between gap-3', compact ? 'pb-2' : 'pb-3')}
              >
                <div className="min-w-0">
                  <p className="font-heading text-sm font-medium">{menuTitle}</p>
                  <p className="text-xs text-muted-foreground">{menuDescription}</p>
                </div>
              </div>
              <SwitcherSeparator />

              <div className={cn('max-h-[24rem] overflow-auto', compact ? 'mt-2' : 'mt-3')}>
                {loading ? (
                  <div className="flex items-center gap-2.5 px-2 py-2 text-sm text-muted-foreground">
                    <SwitcherSpinner />
                    {loadingLabel}
                  </div>
                ) : accounts.length === 0 ? (
                  <div className="px-2 py-2 text-sm text-muted-foreground">{emptyLabel}</div>
                ) : (
                  <div className="flex flex-col gap-1">
                    {accounts.map((account) => {
                      const switching = switchingAccountId === account.id;

                      return (
                        <div key={account.id} className="flex items-center gap-2">
                          <SwitcherButton
                            type="button"
                            variant="ghost"
                            className={cn(
                              'h-auto min-w-0 flex-1 justify-between gap-3 text-left',
                              compact ? 'px-2 py-2' : 'px-3 py-2.5',
                              account.isActive && 'bg-primary/10 text-primary',
                            )}
                            onClick={() => onSelectAccount(account.id)}
                            disabled={switching}
                          >
                            <div
                              className={cn(
                                'flex min-w-0 items-center',
                                compact ? 'gap-2.5' : 'gap-3',
                              )}
                            >
                              <SwitcherAvatar className={compact ? 'size-8' : 'size-9'}>
                                <span className={compact ? 'text-xs' : 'text-sm'}>
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

              <div className={compact ? 'mt-2 pt-2' : 'mt-4 pt-3'}>
                <SwitcherSeparator className={compact ? 'mb-2' : 'mb-3'} />
                <SwitcherButton
                  type="button"
                  className={cn('w-full', compact && 'h-9 rounded-xl')}
                  onClick={onAddAccount}
                >
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
