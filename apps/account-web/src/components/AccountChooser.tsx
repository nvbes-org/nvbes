import { useLocation } from '@tanstack/react-router';
import {
  MultiAccountSwitcher,
  initialsForDisplayName,
  type SharedAccountOption,
} from '@nvbes/web-ui';
import { useMemo, useState } from 'react';
import type { AccountEntry } from '@/lib/account-context';
import { cn } from '@/lib/utils';

interface AccountChooserProps {
  accounts: AccountEntry[];
  loading: boolean;
  className?: string;
  density?: 'default' | 'compact';
}

export function AccountChooser({ accounts, loading, className, density }: AccountChooserProps) {
  const location = useLocation();
  const [switchingTo, setSwitchingTo] = useState<string | null>(null);

  const mappedAccounts = useMemo<SharedAccountOption[]>(
    () =>
      accounts.map((account) => ({
        id: account.authuser,
        email: account.user.email,
        displayName: account.user.display_name,
        isActive: account.session.current,
        avatarFallback: initialsForDisplayName(account.user.display_name),
      })),
    [accounts],
  );

  const handleSwitch = (authuser: string) => {
    setSwitchingTo(authuser);
    const url = new URL(window.location.href);
    url.searchParams.set('authuser', authuser);
    url.searchParams.set('from', location.pathname);
    window.location.assign(url.toString());
  };

  const handleConnectAnotherAccount = () => {
    window.location.assign('/login');
  };

  return (
    <div className={cn('bg-card px-2 py-2', className)}>
      <MultiAccountSwitcher
        accounts={mappedAccounts}
        loading={loading}
        density={density}
        switchingAccountId={switchingTo}
        onSelectAccount={handleSwitch}
        onAddAccount={handleConnectAnotherAccount}
      />
    </div>
  );
}
