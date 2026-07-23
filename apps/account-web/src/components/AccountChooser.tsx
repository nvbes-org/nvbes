import {
  initialsForDisplayName,
  MultiAccountSwitcher,
  type SharedAccountOption,
} from '@nvbes/web-ui';
import { useLocation } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';
import { profileAvatarUrl, subscribeToProfileAvatarUpdates } from '@/account.avatar';
import { accountHrefForAuthuser } from '@/identity.authuser';
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
  const [avatarVersion, setAvatarVersion] = useState(() => Date.now());

  useEffect(() => subscribeToProfileAvatarUpdates(() => setAvatarVersion(Date.now())), []);

  const mappedAccounts = useMemo<SharedAccountOption[]>(
    () =>
      accounts.map((account) => ({
        id: account.authuser,
        email: account.user.email,
        displayName: account.user.display_name,
        isActive: account.session.current,
        avatarFallback: initialsForDisplayName(account.user.display_name),
        avatarUrl: profileAvatarUrl(account.authuser, avatarVersion),
      })),
    [accounts, avatarVersion],
  );

  const handleSwitch = (authuser: string) => {
    setSwitchingTo(authuser);
    window.location.assign(accountHrefForAuthuser(authuser, location.pathname, location.searchStr));
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
        hideCurrentAccountDetailsOnMobile
        switchingAccountId={switchingTo}
        onSelectAccount={handleSwitch}
        onAddAccount={handleConnectAnotherAccount}
      />
    </div>
  );
}
