import {
  revokeServiceAccountOAuthClient,
  rotateServiceAccountOAuthClientSecret,
  type ServiceAccountClient,
} from '../identity.service-accounts.api';
import type { WorkspaceServiceAccountsActionOptions } from './useWorkspaceServiceAccountsPage.actions.shared';

type WorkspaceServiceAccountsClientActionOptions = Pick<
  WorkspaceServiceAccountsActionOptions,
  'workspaceId' | 'selectedServiceAccount' | 'refetch' | 'forms'
>;

export function buildRevokeClientAction({
  workspaceId,
  selectedServiceAccount,
  refetch,
  forms,
}: WorkspaceServiceAccountsClientActionOptions) {
  return async (client: ServiceAccountClient) => {
    if (!workspaceId || !selectedServiceAccount) return;
    const serviceAccount = selectedServiceAccount;
    forms.setBusyAction(`revoke-${client.client_id}`);
    forms.setEditError(null);
    try {
      await revokeServiceAccountOAuthClient(
        workspaceId,
        serviceAccount.principal_id,
        client.client_id,
      );
      await refetch();
    } catch {
      forms.setEditError('Impossible de revoquer ce client OAuth.');
    } finally {
      forms.setBusyAction(null);
    }
  };
}

export function buildRotateClientSecretAction({
  workspaceId,
  selectedServiceAccount,
  refetch,
  forms,
}: WorkspaceServiceAccountsClientActionOptions) {
  return async (client: ServiceAccountClient) => {
    if (!workspaceId || !selectedServiceAccount) return;
    const serviceAccount = selectedServiceAccount;
    forms.setBusyAction(`rotate-${client.client_id}`);
    forms.setEditError(null);
    try {
      const rotated = await rotateServiceAccountOAuthClientSecret(
        workspaceId,
        serviceAccount.principal_id,
        client.client_id,
      );
      forms.setSecretResult({
        kind: 'rotated',
        client_id: rotated.client_id,
        client_secret: rotated.client_secret,
        timestamp: rotated.rotated_at,
      });
      forms.setSecretDialogOpen(true);
      await refetch();
    } catch {
      forms.setEditError('Impossible de rotationner le secret de ce client OAuth.');
    } finally {
      forms.setBusyAction(null);
    }
  };
}
