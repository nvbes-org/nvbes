import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { canManageServiceAccounts } from '@/lib/workspace-permissions';
import { listServiceAccounts } from '../identity.service-accounts.api';
import { useWorkspaceServiceAccountsPageActions } from './useWorkspaceServiceAccountsPage.actions';
import { useWorkspaceServiceAccountsPageForms } from './useWorkspaceServiceAccountsPage.forms';

export function useWorkspaceServiceAccountsPage() {
  const { me, workspaces, loading } = useAccountContext();
  const [workspaceId, setWorkspaceId] = useState('');
  const [selectedServiceAccountId, setSelectedServiceAccountId] = useState('');
  const forms = useWorkspaceServiceAccountsPageForms();

  useEffect(() => {
    const nextWorkspaceId = me?.current_workspace_id ?? workspaces[0]?.id ?? '';
    if (nextWorkspaceId && nextWorkspaceId !== workspaceId) {
      setWorkspaceId(nextWorkspaceId);
    }
  }, [me?.current_workspace_id, workspaces, workspaceId]);

  const workspace = workspaces.find((entry) => entry.id === workspaceId) ?? null;
  const canManage = canManageServiceAccounts(workspace?.role);

  const {
    data: serviceAccounts = [],
    isLoading,
    refetch,
  } = useQuery({
    queryKey: ['identity', 'service-accounts', workspaceId],
    queryFn: ({ signal }) => listServiceAccounts(workspaceId, { signal }),
    enabled: Boolean(workspaceId && canManage),
    staleTime: 5 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    if (serviceAccounts.length === 0) {
      setSelectedServiceAccountId('');
      return;
    }

    const selectedExists = serviceAccounts.some(
      (serviceAccount) => serviceAccount.principal_id === selectedServiceAccountId,
    );

    if (!selectedExists) {
      setSelectedServiceAccountId(serviceAccounts[0].principal_id);
    }
  }, [serviceAccounts, selectedServiceAccountId]);

  const selectedServiceAccount =
    serviceAccounts.find(
      (serviceAccount) => serviceAccount.principal_id === selectedServiceAccountId,
    ) ?? null;

  useEffect(() => {
    if (!selectedServiceAccount) return;
    forms.setUpdateName(selectedServiceAccount.name);
    forms.setUpdateDescription(selectedServiceAccount.description ?? '');
    forms.setUpdateRole(selectedServiceAccount.role);
  }, [selectedServiceAccount]);

  const serviceAccountsCount = serviceAccounts.length;
  const activeCount = serviceAccounts.filter((account) => account.status === 'active').length;
  const clientCount = serviceAccounts.reduce(
    (total, account) => total + account.oauth_clients.length,
    0,
  );

  const handleWorkspaceChange = (value: string) => {
    setWorkspaceId(value);
    setSelectedServiceAccountId('');
  };
  const {
    handleCreateServiceAccount,
    handleUpdateServiceAccount,
    handleToggleLifecycle,
    handleCreateClient,
    handleAttachClient,
    handleRevokeClient,
    handleRotateClientSecret,
  } = useWorkspaceServiceAccountsPageActions({
    workspaceId,
    selectedServiceAccount,
    setSelectedServiceAccountId,
    refetch,
    forms,
  });

  return {
    loading,
    workspaces,
    workspace,
    workspaceId,
    canManage,
    serviceAccounts,
    isLoading,
    refetch,
    selectedServiceAccountId,
    setSelectedServiceAccountId,
    selectedServiceAccount,
    serviceAccountsCount,
    activeCount,
    clientCount,
    ...forms,
    handleWorkspaceChange,
    handleCreateServiceAccount,
    handleUpdateServiceAccount,
    handleToggleLifecycle,
    handleCreateClient,
    handleAttachClient,
    handleRevokeClient,
    handleRotateClientSecret,
  };
}
