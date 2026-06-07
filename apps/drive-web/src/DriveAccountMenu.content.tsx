import { DriveAccountMenuActions } from './DriveAccountMenu.actions';
import { DriveAccountMenuActiveUser } from './DriveAccountMenu.active';
import { DriveAccountMenuOtherAccounts } from './DriveAccountMenu.sessions';
import type { DriveAccountMenuPopoverProps } from './DriveAccountMenu.types';

export function DriveAccountMenuPopover({
  initials,
  user,
  otherSessions,
  sessions,
  onClose,
  onSwitchAccount,
  onAddAccount,
  onRemoveAccount,
  onLogout,
  onLogoutAll,
}: DriveAccountMenuPopoverProps) {
  return (
    <>
      <div className="fixed inset-0 z-40 cursor-default" onClick={onClose} />
      <div className="absolute right-0 z-50 mt-2 w-80 animate-in fade-in-50 slide-in-from-top-1 rounded-xl border border-border/80 bg-popover p-2 text-popover-foreground shadow-lg ring-1 ring-black/5 duration-100">
        <DriveAccountMenuActiveUser initials={initials} user={user} />
        <DriveAccountMenuOtherAccounts
          sessions={otherSessions}
          onSwitchAccount={onSwitchAccount}
          onRemoveAccount={onRemoveAccount}
        />
        <DriveAccountMenuActions
          sessionCount={sessions.length}
          onAddAccount={onAddAccount}
          onLogout={onLogout}
          onLogoutAll={onLogoutAll}
        />
      </div>
    </>
  );
}
