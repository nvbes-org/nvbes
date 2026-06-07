import {
  createServiceAccount,
  reactivateServiceAccount,
  suspendServiceAccount,
  updateServiceAccount,
} from '../identity.service-accounts.api';
import { hasWorkspaceScope } from './useWorkspaceServiceAccountsPage.actions.shared';
import type { WorkspaceServiceAccountsActionOptions } from './useWorkspaceServiceAccountsPage.actions.shared';

export function useWorkspaceServiceAccountsPageAccountActions({
  workspaceId,
  selectedServiceAccount,
  setSelectedServiceAccountId,
  refetch,
  forms,
}: WorkspaceServiceAccountsActionOptions) {
  const handleCreateServiceAccount = async () => {
    if (!hasWorkspaceScope(workspaceId)) return;
    if (!forms.createName.trim()) {
      forms.setCreateError('Le nom du service account est requis.');
      return;
    }

    forms.setBusyAction('create-service-account');
    forms.setCreateError(null);
    try {
      const created = await createServiceAccount(workspaceId, {
        name: forms.createName.trim(),
        description: forms.createDescription.trim() || null,
        role: forms.createRole || null,
      });
      setSelectedServiceAccountId(created.principal_id);
      forms.setCreateOpen(false);
      forms.clearCreate();
      await refetch();
    } catch {
      forms.setCreateError('Impossible de creer le service account.');
    } finally {
      forms.setBusyAction(null);
    }
  };

  const handleUpdateServiceAccount = async () => {
    if (!workspaceId || !selectedServiceAccount) return;
    const serviceAccount = selectedServiceAccount;

    forms.setBusyAction('update-service-account');
    forms.setEditError(null);
    try {
      await updateServiceAccount(workspaceId, serviceAccount.principal_id, {
        name: forms.updateName.trim() || null,
        description: forms.updateDescription.trim() || null,
        role: forms.updateRole || null,
      });
      await refetch();
    } catch {
      forms.setEditError('Impossible de mettre a jour ce service account.');
    } finally {
      forms.setBusyAction(null);
    }
  };

  const handleToggleLifecycle = async () => {
    if (!workspaceId || !selectedServiceAccount) return;
    const serviceAccount = selectedServiceAccount;

    const nextAction =
      serviceAccount.status === 'active' ? 'suspend-service-account' : 'reactivate-service-account';
    forms.setBusyAction(nextAction);
    forms.setEditError(null);
    try {
      if (serviceAccount.status === 'active') {
        await suspendServiceAccount(workspaceId, serviceAccount.principal_id);
      } else {
        await reactivateServiceAccount(workspaceId, serviceAccount.principal_id);
      }
      await refetch();
    } catch {
      forms.setEditError('Impossible de changer le statut de ce service account.');
    } finally {
      forms.setBusyAction(null);
    }
  };

  return {
    handleCreateServiceAccount,
    handleUpdateServiceAccount,
    handleToggleLifecycle,
  };
}
