import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useTransition } from 'react';
import { logoutDriveSession, startDriveLogin } from './drive.auth.functions';
import { driveQueryKeys } from './drive.queries';
import { clearDriveSession, getSessions, removeSession, setActiveSession } from './drive.session';

export function useDriveAccountMenu({ accessToken }: { accessToken: string }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [_isPending, startTransition] = useTransition();

  const sessions = getSessions();

  async function handleLogout() {
    await logoutDriveSession(accessToken);
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    startTransition(() => {
      void navigate({ to: '/', replace: true });
    });
  }

  function handleSwitchAccount(userId: string) {
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    setActiveSession(userId);
  }

  async function handleAddAccount() {
    await startDriveLogin();
  }

  function handleRemoveAccount(userId: string) {
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    removeSession(userId);
  }

  async function handleLogoutAll() {
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    await logoutDriveSession(accessToken);
    clearDriveSession();
    startTransition(() => {
      void navigate({ to: '/', replace: true });
    });
  }

  return {
    sessions,
    handleAddAccount,
    handleLogout,
    handleLogoutAll,
    handleRemoveAccount,
    handleSwitchAccount,
  };
}
