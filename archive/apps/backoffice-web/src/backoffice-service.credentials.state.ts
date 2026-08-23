import { getSafeLocalStorage } from '@nvbes/web-runtime';
import type { AdminCredentials, BackofficeRole } from './backoffice-service.types';

const storageKey = 'nvbes.backoffice-service.credentials';

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
      backofficeRole: normalizeBackofficeRole(parsed.backofficeRole),
      secondApproverPrincipalId:
        typeof parsed.secondApproverPrincipalId === 'string'
          ? parsed.secondApproverPrincipalId
          : '',
      secondApproverRole: 'platform_admin',
    };
  } catch {
    return emptyCredentials;
  }
}

function normalizeBackofficeRole(value: unknown): BackofficeRole {
  return backofficeRoles.includes(value as BackofficeRole) ? (value as BackofficeRole) : 'viewer';
}

const backofficeRoles: BackofficeRole[] = [
  'compliance_admin',
  'developer_admin',
  'finance_admin',
  'operations_admin',
  'platform_admin',
  'product_admin',
  'security_admin',
  'support_agent',
  'viewer',
];

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
