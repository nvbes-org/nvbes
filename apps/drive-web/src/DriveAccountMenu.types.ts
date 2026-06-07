import type { MouseEvent } from 'react';
import type { DriveMeResponse } from './drive.api';
import type { AccountSession } from './drive.session.client';

export type DriveUser = DriveMeResponse['user'];

export type DriveAccountMenuSessionHandler = (userId: string) => void;
export type DriveAccountMenuRemoveHandler = (event: MouseEvent, userId: string) => void;
export type DriveAccountMenuAsyncHandler = () => Promise<void>;
export type DriveAccountMenuCloseHandler = () => void;

export type DriveAccountMenuButtonProps = {
  initials: string;
  user: DriveUser;
  onToggle: () => void;
};

export type DriveAccountMenuPopoverProps = {
  initials: string;
  user: DriveUser;
  otherSessions: AccountSession[];
  sessions: AccountSession[];
  onClose: DriveAccountMenuCloseHandler;
  onSwitchAccount: DriveAccountMenuSessionHandler;
  onAddAccount: DriveAccountMenuAsyncHandler;
  onRemoveAccount: DriveAccountMenuRemoveHandler;
  onLogout: DriveAccountMenuAsyncHandler;
  onLogoutAll: DriveAccountMenuAsyncHandler;
};
