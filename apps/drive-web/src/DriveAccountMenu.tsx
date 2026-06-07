import type { DriveMeResponse } from './drive.api';
import { DriveAccountMenuButton, DriveAccountMenuPopover } from './DriveAccountMenu.shared';
import { useDriveAccountMenu } from './useDriveAccountMenu';

type DriveUser = DriveMeResponse['user'];

export function DriveAccountMenu({ accessToken, user }: { accessToken: string; user: DriveUser }) {
  const {
    initials,
    isOpen,
    otherSessions,
    sessions,
    setIsOpen,
    handleAddAccount,
    handleLogout,
    handleLogoutAll,
    handleRemoveAccount,
    handleSwitchAccount,
  } = useDriveAccountMenu({ accessToken, user });

  return (
    <div className="relative">
      <DriveAccountMenuButton initials={initials} user={user} onToggle={() => setIsOpen(!isOpen)} />

      {isOpen ? (
        <DriveAccountMenuPopover
          initials={initials}
          user={user}
          otherSessions={otherSessions}
          sessions={sessions}
          onClose={() => setIsOpen(false)}
          onSwitchAccount={handleSwitchAccount}
          onAddAccount={handleAddAccount}
          onRemoveAccount={handleRemoveAccount}
          onLogout={handleLogout}
          onLogoutAll={handleLogoutAll}
        />
      ) : null}
    </div>
  );
}
