import { getSafeLocalStorage } from '@nvbes/web-runtime';
import type { AdminCredentials } from './internal-admin.types';

const storageKey = 'nvbes.internal-admin.credentials';

export const emptyCredentials: AdminCredentials = {
  workspaceId: '',
  internalToken: '',
  actorPrincipalId: '',
  backofficeRole: 'platform_admin',
  secondApproverPrincipalId: '',
  secondApproverRole: 'platform_admin',
};

export function loadCredentials(): AdminCredentials {
  try {
    const raw = getSafeLocalStorage().getItem(storageKey);
    if (!raw) return emptyCredentials;
    const parsed = JSON.parse(raw) as Partial<AdminCredentials>;
    return {
      workspaceId: typeof parsed.workspaceId === 'string' ? parsed.workspaceId : '',
      internalToken: typeof parsed.internalToken === 'string' ? parsed.internalToken : '',
      actorPrincipalId: typeof parsed.actorPrincipalId === 'string' ? parsed.actorPrincipalId : '',
      backofficeRole:
        typeof parsed.backofficeRole === 'string' ? parsed.backofficeRole : 'platform_admin',
      secondApproverPrincipalId:
        typeof parsed.secondApproverPrincipalId === 'string'
          ? parsed.secondApproverPrincipalId
          : '',
      secondApproverRole:
        typeof parsed.secondApproverRole === 'string'
          ? parsed.secondApproverRole
          : 'platform_admin',
    };
  } catch {
    return emptyCredentials;
  }
}

export function saveCredentials(credentials: AdminCredentials) {
  getSafeLocalStorage().setItem(storageKey, JSON.stringify(credentials));
}

export function credentialsReady(credentials: AdminCredentials): boolean {
  return (
    credentials.workspaceId.trim().length > 0 &&
    credentials.internalToken.trim().length > 0 &&
    credentials.actorPrincipalId.trim().length > 0 &&
    credentials.backofficeRole.trim().length > 0
  );
}
