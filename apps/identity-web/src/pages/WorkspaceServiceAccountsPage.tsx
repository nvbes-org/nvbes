import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { canManageServiceAccounts } from '@/lib/workspace-permissions';
import {
  attachOAuthClient,
  type CreateServiceAccountOAuthClientInput,
  createServiceAccount,
  createServiceAccountOAuthClient,
  listServiceAccounts,
  reactivateServiceAccount,
  revokeServiceAccountOAuthClient,
  rotateServiceAccountOAuthClientSecret,
  type ServiceAccountClient,
  suspendServiceAccount,
  updateServiceAccount,
} from '../identity.service-accounts.api';
import {
  driveScopesPreset,
  type SecretResult,
  splitList,
} from './WorkspaceServiceAccounts.helpers';
import { WorkspaceServiceAccountsAccessDenied } from './WorkspaceServiceAccountsAccessDenied';
import { WorkspaceServiceAccountsDetailCard } from './WorkspaceServiceAccountsDetailCard';
import { WorkspaceServiceAccountsDialogs } from './WorkspaceServiceAccountsDialogs';
import { WorkspaceServiceAccountsHero } from './WorkspaceServiceAccountsHero';
import { WorkspaceServiceAccountsListCard } from './WorkspaceServiceAccountsListCard';

function ServiceAccountsSkeleton() {
  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up">
      <div className="rounded-3xl border border-border/70 bg-card p-6">
        <div className="h-6 w-48 rounded bg-muted" />
        <div className="mt-2 h-4 w-80 rounded bg-muted" />
        <div className="mt-6 grid gap-3 sm:grid-cols-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="h-20 rounded-2xl bg-muted" />
          ))}
        </div>
      </div>
      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <div className="h-[28rem] rounded-3xl bg-muted" />
        <div className="h-[28rem] rounded-3xl bg-muted" />
      </div>
    </div>
  );
}

export default function WorkspaceServiceAccountsPage() {
  const { me, workspaces, loading } = useAccountContext();
  const [workspaceId, setWorkspaceId] = useState('');
  const [selectedServiceAccountId, setSelectedServiceAccountId] = useState('');

  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState('');
  const [createDescription, setCreateDescription] = useState('');
  const [createRole, setCreateRole] = useState('member');
  const [createError, setCreateError] = useState<string | null>(null);

  const [clientDialogOpen, setClientDialogOpen] = useState(false);
  const [clientName, setClientName] = useState('');
  const [clientScopes, setClientScopes] = useState(driveScopesPreset);
  const [clientAudiences, setClientAudiences] = useState('nvbes-drive-api');
  const [clientResources, setClientResources] = useState('');
  const [clientRequiredAcr, setClientRequiredAcr] = useState('');
  const [clientAssertionRequired, setClientAssertionRequired] = useState(false);
  const [clientAssertionJwk, setClientAssertionJwk] = useState('');
  const [clientError, setClientError] = useState<string | null>(null);

  const [attachDialogOpen, setAttachDialogOpen] = useState(false);
  const [attachClientId, setAttachClientId] = useState('');
  const [attachError, setAttachError] = useState<string | null>(null);

  const [editError, setEditError] = useState<string | null>(null);

  const [updateName, setUpdateName] = useState('');
  const [updateDescription, setUpdateDescription] = useState('');
  const [updateRole, setUpdateRole] = useState('member');

  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [secretResult, setSecretResult] = useState<SecretResult | null>(null);
  const [secretDialogOpen, setSecretDialogOpen] = useState(false);

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
    setUpdateName(selectedServiceAccount.name);
    setUpdateDescription(selectedServiceAccount.description ?? '');
    setUpdateRole(selectedServiceAccount.role);
  }, [selectedServiceAccount]);

  const serviceAccountsCount = serviceAccounts.length;
  const activeCount = serviceAccounts.filter((account) => account.status === 'active').length;
  const clientCount = serviceAccounts.reduce(
    (total, account) => total + account.oauth_clients.length,
    0,
  );

  const clearCreate = () => {
    setCreateName('');
    setCreateDescription('');
    setCreateRole('member');
    setCreateError(null);
  };

  const clearClient = () => {
    setClientName('');
    setClientScopes(driveScopesPreset);
    setClientAudiences('nvbes-drive-api');
    setClientResources('');
    setClientRequiredAcr('');
    setClientAssertionRequired(false);
    setClientAssertionJwk('');
    setClientError(null);
  };

  const clearAttach = () => {
    setAttachClientId('');
    setAttachError(null);
  };

  const handleWorkspaceChange = (value: string) => {
    setWorkspaceId(value);
    setSelectedServiceAccountId('');
  };

  const handleCreateServiceAccount = async () => {
    if (!workspaceId) return;
    if (!createName.trim()) {
      setCreateError('Le nom du service account est requis.');
      return;
    }

    setBusyAction('create-service-account');
    setCreateError(null);
    try {
      const created = await createServiceAccount(workspaceId, {
        name: createName.trim(),
        description: createDescription.trim() || null,
        role: createRole || null,
      });
      setSelectedServiceAccountId(created.principal_id);
      setCreateOpen(false);
      clearCreate();
      await refetch();
    } catch {
      setCreateError('Impossible de creer le service account.');
    } finally {
      setBusyAction(null);
    }
  };

  const handleUpdateServiceAccount = async () => {
    if (!workspaceId || !selectedServiceAccount) return;

    setBusyAction('update-service-account');
    setEditError(null);
    try {
      await updateServiceAccount(workspaceId, selectedServiceAccount.principal_id, {
        name: updateName.trim() || null,
        description: updateDescription.trim() || null,
        role: updateRole || null,
      });
      await refetch();
    } catch {
      setEditError('Impossible de mettre a jour ce service account.');
    } finally {
      setBusyAction(null);
    }
  };

  const handleToggleLifecycle = async () => {
    if (!workspaceId || !selectedServiceAccount) return;

    const nextAction =
      selectedServiceAccount.status === 'active'
        ? 'suspend-service-account'
        : 'reactivate-service-account';
    setBusyAction(nextAction);
    setEditError(null);
    try {
      if (selectedServiceAccount.status === 'active') {
        await suspendServiceAccount(workspaceId, selectedServiceAccount.principal_id);
      } else {
        await reactivateServiceAccount(workspaceId, selectedServiceAccount.principal_id);
      }
      await refetch();
    } catch {
      setEditError('Impossible de changer le statut de ce service account.');
    } finally {
      setBusyAction(null);
    }
  };

  const handleCreateClient = async () => {
    if (!workspaceId || !selectedServiceAccount) return;
    if (!clientName.trim()) {
      setClientError('Le nom du client OAuth est requis.');
      return;
    }

    const allowedScopes = splitList(clientScopes);
    if (allowedScopes.length === 0) {
      setClientError('Au moins un scope OAuth est requis.');
      return;
    }

    let clientAssertionPublicKeyJwk: unknown;
    if (clientAssertionJwk.trim()) {
      try {
        clientAssertionPublicKeyJwk = JSON.parse(clientAssertionJwk);
      } catch {
        setClientError('Le JWK doit etre du JSON valide.');
        return;
      }
    }

    setBusyAction('create-client');
    setClientError(null);
    try {
      const result = await createServiceAccountOAuthClient(
        workspaceId,
        selectedServiceAccount.principal_id,
        {
          name: clientName.trim(),
          allowed_scopes: allowedScopes,
          allowed_audiences: splitList(clientAudiences),
          allowed_resources: splitList(clientResources),
          required_acr: clientRequiredAcr.trim() || null,
          client_assertion_required: clientAssertionRequired,
          client_assertion_public_key_jwk: clientAssertionPublicKeyJwk,
        } satisfies CreateServiceAccountOAuthClientInput,
      );
      setSecretResult({
        kind: 'created',
        client_id: result.client.client_id,
        client_secret: result.client_secret,
        timestamp: result.client.created_at,
      });
      setSecretDialogOpen(true);
      setClientDialogOpen(false);
      clearClient();
      await refetch();
    } finally {
      setBusyAction(null);
    }
  };

  const handleAttachClient = async () => {
    if (!workspaceId || !selectedServiceAccount) return;
    if (!attachClientId.trim()) {
      setAttachError('Le client_id est requis.');
      return;
    }

    setBusyAction('attach-client');
    setAttachError(null);
    try {
      await attachOAuthClient(workspaceId, selectedServiceAccount.principal_id, {
        client_id: attachClientId.trim(),
      });
      setAttachDialogOpen(false);
      clearAttach();
      await refetch();
    } catch {
      setAttachError("Impossible d'attacher ce client OAuth.");
    } finally {
      setBusyAction(null);
    }
  };

  const handleRevokeClient = async (client: ServiceAccountClient) => {
    if (!workspaceId || !selectedServiceAccount) return;
    setBusyAction(`revoke-${client.client_id}`);
    setEditError(null);
    try {
      await revokeServiceAccountOAuthClient(
        workspaceId,
        selectedServiceAccount.principal_id,
        client.client_id,
      );
      await refetch();
    } catch {
      setEditError('Impossible de revoquer ce client OAuth.');
    } finally {
      setBusyAction(null);
    }
  };

  const handleRotateClientSecret = async (client: ServiceAccountClient) => {
    if (!workspaceId || !selectedServiceAccount) return;
    setBusyAction(`rotate-${client.client_id}`);
    setEditError(null);
    try {
      const rotated = await rotateServiceAccountOAuthClientSecret(
        workspaceId,
        selectedServiceAccount.principal_id,
        client.client_id,
      );
      setSecretResult({
        kind: 'rotated',
        client_id: rotated.client_id,
        client_secret: rotated.client_secret,
        timestamp: rotated.rotated_at,
      });
      setSecretDialogOpen(true);
      await refetch();
    } catch {
      setEditError('Impossible de rotationner le secret de ce client OAuth.');
    } finally {
      setBusyAction(null);
    }
  };

  if (loading) return <ServiceAccountsSkeleton />;

  if (workspaceId && !canManage) {
    return <WorkspaceServiceAccountsAccessDenied workspace={workspace} workspaceId={workspaceId} />;
  }

  if (isLoading) return <ServiceAccountsSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up">
      <WorkspaceServiceAccountsHero
        workspace={workspace}
        workspaceId={workspaceId}
        workspaces={workspaces}
        canManage={canManage}
        serviceAccountsCount={serviceAccountsCount}
        activeCount={activeCount}
        clientCount={clientCount}
        onWorkspaceChange={handleWorkspaceChange}
        onCreate={() => {
          clearCreate();
          setCreateOpen(true);
        }}
        onRefresh={() => void refetch()}
      />

      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <WorkspaceServiceAccountsListCard
          serviceAccounts={serviceAccounts}
          selectedServiceAccountId={selectedServiceAccountId}
          onSelect={setSelectedServiceAccountId}
        />

        <WorkspaceServiceAccountsDetailCard
          selectedServiceAccount={selectedServiceAccount}
          updateName={updateName}
          setUpdateName={setUpdateName}
          updateDescription={updateDescription}
          setUpdateDescription={setUpdateDescription}
          updateRole={updateRole}
          setUpdateRole={setUpdateRole}
          busyAction={busyAction}
          editError={editError}
          onSave={() => void handleUpdateServiceAccount()}
          onToggleLifecycle={() => void handleToggleLifecycle()}
          onOpenCreateClient={() => {
            clearClient();
            setClientDialogOpen(true);
          }}
          onOpenAttachClient={() => {
            clearAttach();
            setAttachDialogOpen(true);
          }}
          onRotateClientSecret={(client) => void handleRotateClientSecret(client)}
          onRevokeClient={(client) => void handleRevokeClient(client)}
        />
      </div>

      <WorkspaceServiceAccountsDialogs
        createOpen={createOpen}
        setCreateOpen={setCreateOpen}
        createName={createName}
        setCreateName={setCreateName}
        createDescription={createDescription}
        setCreateDescription={setCreateDescription}
        createRole={createRole}
        setCreateRole={setCreateRole}
        createError={createError}
        busyAction={busyAction}
        onCreate={() => void handleCreateServiceAccount()}
        onClearCreate={clearCreate}
        clientDialogOpen={clientDialogOpen}
        setClientDialogOpen={setClientDialogOpen}
        clientName={clientName}
        setClientName={setClientName}
        clientScopes={clientScopes}
        setClientScopes={setClientScopes}
        clientAudiences={clientAudiences}
        setClientAudiences={setClientAudiences}
        clientResources={clientResources}
        setClientResources={setClientResources}
        clientRequiredAcr={clientRequiredAcr}
        setClientRequiredAcr={setClientRequiredAcr}
        clientAssertionRequired={clientAssertionRequired}
        setClientAssertionRequired={setClientAssertionRequired}
        clientAssertionJwk={clientAssertionJwk}
        setClientAssertionJwk={setClientAssertionJwk}
        clientError={clientError}
        onCreateClient={() => void handleCreateClient()}
        onClearClient={clearClient}
        attachDialogOpen={attachDialogOpen}
        setAttachDialogOpen={setAttachDialogOpen}
        attachClientId={attachClientId}
        setAttachClientId={setAttachClientId}
        attachError={attachError}
        onAttachClient={() => void handleAttachClient()}
        onClearAttach={clearAttach}
        secretResult={secretResult}
        secretDialogOpen={secretDialogOpen}
        setSecretDialogOpen={setSecretDialogOpen}
      />
    </div>
  );
}
