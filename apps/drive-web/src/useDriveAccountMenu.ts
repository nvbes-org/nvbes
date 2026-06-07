import { runClientEffect } from '@nvbes/web-runtime';
import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import type { MouseEvent } from 'react';
import { useMemo, useState, useTransition } from 'react';
import type { DriveMeResponse } from './drive.api';
import { driveQueryKeys } from './drive.queries';
import { clearDriveSession, getSessions, removeSession, setActiveSession } from './drive.session';
import { logoutDriveWorkflow, startDriveLoginWorkflow } from './drive.workflow';

type DriveUser = DriveMeResponse['user'];

export function initialsFor(name: string): string {
  return name
    .split(' ')
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('');
}

export function useDriveAccountMenu({
  accessToken,
  user,
}: {
  accessToken: string;
  user: DriveUser;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [_isPending, startTransition] = useTransition();
  const [isOpen, setIsOpen] = useState(false);

  const sessions = getSessions();
  const otherSessions = useMemo(
    () => sessions.filter((session) => session.userId !== user.id),
    [sessions, user.id],
  );
  const initials = initialsFor(user.display_name);

  async function handleLogout() {
    await runClientEffect(logoutDriveWorkflow(accessToken));
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    startTransition(() => {
      void navigate({ to: '/', replace: true });
    });
  }

  function handleSwitchAccount(userId: string) {
    setIsOpen(false);
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    setActiveSession(userId);
  }

  async function handleAddAccount() {
    setIsOpen(false);
    await runClientEffect(startDriveLoginWorkflow());
  }

  function handleRemoveAccount(event: MouseEvent, userId: string) {
    event.stopPropagation();
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    removeSession(userId);
  }

  async function handleLogoutAll() {
    setIsOpen(false);
    queryClient.removeQueries({ queryKey: driveQueryKeys.all });
    await runClientEffect(logoutDriveWorkflow(accessToken));
    clearDriveSession();
    startTransition(() => {
      void navigate({ to: '/', replace: true });
    });
  }

  return {
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
  };
}
