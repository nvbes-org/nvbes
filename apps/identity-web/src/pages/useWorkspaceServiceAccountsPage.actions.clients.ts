import {
  buildAttachClientAction,
  buildCreateClientAction,
} from './useWorkspaceServiceAccountsPage.actions.clients.create';
import {
  buildRevokeClientAction,
  buildRotateClientSecretAction,
} from './useWorkspaceServiceAccountsPage.actions.clients.manage';
import type { WorkspaceServiceAccountsActionOptions } from './useWorkspaceServiceAccountsPage.actions.shared';

export function useWorkspaceServiceAccountsPageClientActions({
  workspaceId,
  selectedServiceAccount,
  refetch,
  forms,
}: WorkspaceServiceAccountsActionOptions) {
  const options = { workspaceId, selectedServiceAccount, refetch, forms };
  const handleCreateClient = buildCreateClientAction(options);
  const handleAttachClient = buildAttachClientAction(options);
  const handleRevokeClient = buildRevokeClientAction(options);
  const handleRotateClientSecret = buildRotateClientSecretAction(options);

  return {
    handleCreateClient,
    handleAttachClient,
    handleRevokeClient,
    handleRotateClientSecret,
  };
}
