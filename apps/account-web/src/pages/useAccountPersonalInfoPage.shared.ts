import type { AccountEntry, AccountMe, AccountPrincipal } from '@nvbes/identity-client';
import { accountQueryKeys } from '@/account.queries';

export interface AccountPersonalInfoQueryData {
  user: AccountPrincipal;
}

export function getAccountPersonalInfoQueryKey(authuser: string) {
  return accountQueryKeys.personalInfo(authuser);
}

export function getAccountPersonalInfoContextUpdate(data: { user: AccountPrincipal }) {
  return (
    current:
      | {
          me: AccountMe | null;
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
