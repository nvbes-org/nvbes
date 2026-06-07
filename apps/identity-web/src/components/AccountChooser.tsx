import { useLocation } from '@tanstack/react-router';
import { useEffect, useMemo, useRef, useState } from 'react';

import {
  AccountChooserMenu,
  AccountChooserTrigger,
  type AccountChooserProps,
} from './AccountChooser.shared';

export function AccountChooser({ accounts, loading }: AccountChooserProps) {
  const location = useLocation();
  const rootRef = useRef<HTMLDivElement | null>(null);
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
      <AccountChooserTrigger
        currentAccount={currentAccount}
        open={open}
        onToggle={() => setOpen((value) => !value)}
      />

      {open && (
        <AccountChooserMenu
          accounts={accounts}
          loading={loading}
          switchingTo={switchingTo}
          onSwitch={handleSwitch}
        />
      )}
    </div>
  );
}
