import { WorkspaceServiceAccountsCreateDialog } from './WorkspaceServiceAccountsCreateDialog';
import type { WorkspaceServiceAccountsDialogsProps } from './WorkspaceServiceAccountsDialogs.types';

export function WorkspaceServiceAccountsCreateDialogs({
  createOpen,
  setCreateOpen,
  createName,
  setCreateName,
  createDescription,
  setCreateDescription,
  createRole,
  setCreateRole,
  createError,
  busyAction,
  onCreate,
  onClearCreate,
}: Pick<
  WorkspaceServiceAccountsDialogsProps,
  | 'createOpen'
  | 'setCreateOpen'
  | 'createName'
  | 'setCreateName'
  | 'createDescription'
  | 'setCreateDescription'
  | 'createRole'
  | 'setCreateRole'
  | 'createError'
  | 'busyAction'
  | 'onCreate'
  | 'onClearCreate'
>) {
  return (
    <WorkspaceServiceAccountsCreateDialog
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
      onCreate={onCreate}
      onClearCreate={onClearCreate}
    />
  );
}
