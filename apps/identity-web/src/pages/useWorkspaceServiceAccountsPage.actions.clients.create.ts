import {
  attachOAuthClient,
  createServiceAccountOAuthClient,
  type CreateServiceAccountOAuthClientInput,
} from '../identity.service-accounts.api';
import { splitList } from './WorkspaceServiceAccounts.helpers';
import type { WorkspaceServiceAccountsActionOptions } from './useWorkspaceServiceAccountsPage.actions.shared';

type WorkspaceServiceAccountsClientActionOptions = Pick<
  WorkspaceServiceAccountsActionOptions,
  'workspaceId' | 'selectedServiceAccount' | 'refetch' | 'forms'
>;

export function buildCreateClientAction({
  workspaceId,
  selectedServiceAccount,
  refetch,
  forms,
}: WorkspaceServiceAccountsClientActionOptions) {
  return async () => {
    if (!workspaceId || !selectedServiceAccount) return;
    const serviceAccount = selectedServiceAccount;
    if (!forms.clientName.trim()) {
      forms.setClientError('Le nom du client OAuth est requis.');
      return;
    }

    const allowedScopes = splitList(forms.clientScopes);
    if (allowedScopes.length === 0) {
      forms.setClientError('Au moins un scope OAuth est requis.');
      return;
    }

    let clientAssertionPublicKeyJwk: unknown;
    if (forms.clientAssertionJwk.trim()) {
      try {
        clientAssertionPublicKeyJwk = JSON.parse(forms.clientAssertionJwk);
      } catch {
        forms.setClientError('Le JWK doit etre du JSON valide.');
        return;
      }
    }

    forms.setBusyAction('create-client');
    forms.setClientError(null);
    try {
      const result = await createServiceAccountOAuthClient(
        workspaceId,
        serviceAccount.principal_id,
        {
          name: forms.clientName.trim(),
          allowed_scopes: allowedScopes,
          allowed_audiences: splitList(forms.clientAudiences),
          allowed_resources: splitList(forms.clientResources),
          required_acr: forms.clientRequiredAcr.trim() || null,
          client_assertion_required: forms.clientAssertionRequired,
          client_assertion_public_key_jwk: clientAssertionPublicKeyJwk,
        } satisfies CreateServiceAccountOAuthClientInput,
      );
      forms.setSecretResult({
        kind: 'created',
        client_id: result.client.client_id,
        client_secret: result.client_secret,
        timestamp: result.client.created_at,
      });
      forms.setSecretDialogOpen(true);
      forms.setClientDialogOpen(false);
      forms.clearClient();
      await refetch();
    } finally {
      forms.setBusyAction(null);
    }
  };
}

export function buildAttachClientAction({
  workspaceId,
  selectedServiceAccount,
  refetch,
  forms,
}: WorkspaceServiceAccountsClientActionOptions) {
  return async () => {
    if (!workspaceId || !selectedServiceAccount) return;
    const serviceAccount = selectedServiceAccount;
    if (!forms.attachClientId.trim()) {
      forms.setAttachError('Le client_id est requis.');
      return;
    }

    forms.setBusyAction('attach-client');
    forms.setAttachError(null);
    try {
      await attachOAuthClient(workspaceId, serviceAccount.principal_id, {
        client_id: forms.attachClientId.trim(),
      });
      forms.setAttachDialogOpen(false);
      forms.clearAttach();
      await refetch();
    } catch {
      forms.setAttachError("Impossible d'attacher ce client OAuth.");
    } finally {
      forms.setBusyAction(null);
    }
  };
}
