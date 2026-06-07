import {
  CreateWorkspaceDialog,
  WorkspacesHeader,
  WorkspacesList,
  WorkspacesSkeleton,
} from './WorkspacesPage.shared';
import { useWorkspacesPage } from './useWorkspacesPage';

export default function WorkspacesPage() {
  const {
    createError,
    creating,
    handleCreate,
    handleOpenCreate,
    listRef,
    loading,
    me,
    newWorkspaceName,
    setNewWorkspaceName,
    setShowCreateDialog,
    showCreateDialog,
    totalSize,
    virtualItems,
    virtualizer,
    workspaces,
  } = useWorkspacesPage();

  if (loading) {
    return <WorkspacesSkeleton />;
  }

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <WorkspacesHeader onOpenCreate={handleOpenCreate} />
      <WorkspacesList
        me={me}
        workspaces={workspaces}
        listRef={listRef}
        totalSize={totalSize}
        virtualItems={virtualItems}
        measureElement={virtualizer.measureElement}
        onOpenCreate={handleOpenCreate}
      />
      <CreateWorkspaceDialog
        open={showCreateDialog}
        workspaceName={newWorkspaceName}
        creating={creating}
        error={createError}
        onOpenChange={setShowCreateDialog}
        onWorkspaceNameChange={setNewWorkspaceName}
        onSubmit={handleCreate}
      />
    </div>
  );
}
