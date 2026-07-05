import type {
  AccountEntry,
  AccountMe,
  AccountPrincipal,
  AccountWorkspace,
} from '@nvbes/identity-client';
import { accountQueryKeys } from '@/account.queries';

export interface AccountPersonalInfoQueryData {
  user: AccountPrincipal;
  current_workspace_region: string | null;
  current_workspace_id: string | null;
  workspaces: AccountWorkspace[];
}

export function getAccountPersonalInfoQueryKey(authuser: string) {
  return accountQueryKeys.personalInfo(authuser);
}

export function getAccountPersonalInfoContextUpdate(data: { user: AccountPrincipal }) {
  return (
    current:
      | {
          me: AccountMe | null;
          workspaces: AccountWorkspace[];
          accounts: AccountEntry[];
        }
      | undefined,
  ) =>
    current?.me
      ? {
          ...current,
          me: {
            ...current.me,
            user: data.user,
          },
        }
      : current;
}

export function formatMemberSince(createdAt: string | undefined) {
  if (!createdAt) {
    return '';
  }

  return new Date(createdAt).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}
