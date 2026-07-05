import type { ServiceAccount } from '../identity.service-accounts.api';
import type { WorkspaceServiceAccountsPageForms } from './useWorkspaceServiceAccountsPage.forms';

export type WorkspaceServiceAccountsActionOptions = {
  workspaceId: string;
  selectedServiceAccount: ServiceAccount | null;
  setSelectedServiceAccountId: (value: string) => void;
  refetch: () => Promise<unknown>;
  forms: WorkspaceServiceAccountsPageForms;
};

export function hasWorkspaceScope(workspaceId: string) {
  return Boolean(workspaceId);
}

export function hasSelectedServiceAccount(
  workspaceId: string,
  selectedServiceAccount: ServiceAccount | null,
) {
  return Boolean(workspaceId && selectedServiceAccount);
}
