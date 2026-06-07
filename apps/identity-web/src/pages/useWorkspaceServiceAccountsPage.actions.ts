import type { ServiceAccount } from '../identity.service-accounts.api';
import { useWorkspaceServiceAccountsPageAccountActions } from './useWorkspaceServiceAccountsPage.actions.accounts';
import { useWorkspaceServiceAccountsPageClientActions } from './useWorkspaceServiceAccountsPage.actions.clients';
import type { WorkspaceServiceAccountsPageForms } from './useWorkspaceServiceAccountsPage.forms';

type UseWorkspaceServiceAccountsPageActionsOptions = {
  workspaceId: string;
  selectedServiceAccount: ServiceAccount | null;
  setSelectedServiceAccountId: (value: string) => void;
  refetch: () => Promise<unknown>;
  forms: WorkspaceServiceAccountsPageForms;
};

export function useWorkspaceServiceAccountsPageActions({
  workspaceId,
  selectedServiceAccount,
  setSelectedServiceAccountId,
  refetch,
  forms,
}: UseWorkspaceServiceAccountsPageActionsOptions) {
  const accountActions = useWorkspaceServiceAccountsPageAccountActions({
    workspaceId,
    selectedServiceAccount,
    setSelectedServiceAccountId,
    refetch,
    forms,
  });
  const clientActions = useWorkspaceServiceAccountsPageClientActions({
    workspaceId,
    selectedServiceAccount,
    setSelectedServiceAccountId,
    refetch,
    forms,
  });

  return {
    ...accountActions,
    ...clientActions,
  };
}
