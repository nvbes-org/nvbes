import { WorkspaceServiceAccountsDetailCard } from './WorkspaceServiceAccountsDetailCard';
import { WorkspaceServiceAccountsListCard } from './WorkspaceServiceAccountsListCard';
import type { WorkspaceServiceAccountsPageModel } from './WorkspaceServiceAccountsPage.types';

export function WorkspaceServiceAccountsMainGrid(
  props: Pick<
    WorkspaceServiceAccountsPageModel,
    | 'serviceAccounts'
    | 'selectedServiceAccountId'
    | 'setSelectedServiceAccountId'
    | 'selectedServiceAccount'
    | 'updateName'
    | 'setUpdateName'
    | 'updateDescription'
    | 'setUpdateDescription'
    | 'updateRole'
    | 'setUpdateRole'
    | 'busyAction'
    | 'editError'
    | 'clearClient'
    | 'clearAttach'
    | 'setClientDialogOpen'
    | 'setAttachDialogOpen'
    | 'handleUpdateServiceAccount'
    | 'handleToggleLifecycle'
    | 'handleRotateClientSecret'
    | 'handleRevokeClient'
  >,
) {
  const {
    serviceAccounts,
    selectedServiceAccountId,
    setSelectedServiceAccountId,
    selectedServiceAccount,
    updateName,
    setUpdateName,
    updateDescription,
    setUpdateDescription,
    updateRole,
    setUpdateRole,
    busyAction,
    editError,
    clearClient,
    clearAttach,
    setClientDialogOpen,
    setAttachDialogOpen,
    handleUpdateServiceAccount,
    handleToggleLifecycle,
    handleRotateClientSecret,
    handleRevokeClient,
  } = props;

  return (
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
  );
}
