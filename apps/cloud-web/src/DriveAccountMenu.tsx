import type { DriveMeResponse } from './drive.api';
import {
  MultiAccountSwitcher,
  initialsForDisplayName,
  type SharedAccountOption,
} from '@nvbes/web-ui';
import { DriveAccountMenuActions } from './DriveAccountMenu.actions';
import { useDriveAccountMenu } from './useDriveAccountMenu';

type DriveUser = DriveMeResponse['user'];

export function DriveAccountMenu({ accessToken, user }: { accessToken: string; user: DriveUser }) {
  const {
    sessions,
    handleAddAccount,
    handleLogout,
    handleLogoutAll,
    handleRemoveAccount,
    handleSwitchAccount,
  } = useDriveAccountMenu({ accessToken });

  const accounts: SharedAccountOption[] = sessions.map((session) => ({
    id: session.userId,
    email: session.email,
    displayName: session.name,
    isActive: session.userId === user.id,
    avatarFallback: initialsForDisplayName(session.name),
  }));

  return (
    <MultiAccountSwitcher
      className="w-[13rem] md:w-[14rem]"
      accounts={accounts}
      loading={false}
      onSelectAccount={handleSwitchAccount}
      onAddAccount={() => void handleAddAccount()}
      onRemoveAccount={handleRemoveAccount}
      addAccountLabel="Ajouter un compte"
      footerActions={
        <DriveAccountMenuActions
          sessionCount={sessions.length}
          onLogout={handleLogout}
          onLogoutAll={handleLogoutAll}
        />
      }
    />
  );
}
