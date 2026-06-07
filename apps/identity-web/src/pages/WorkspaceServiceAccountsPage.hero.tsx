import { WorkspaceServiceAccountsHero } from './WorkspaceServiceAccountsHero';
import type { WorkspaceServiceAccountsPageModel } from './WorkspaceServiceAccountsPage.types';

export function WorkspaceServiceAccountsHeroSection(
  props: Pick<
    WorkspaceServiceAccountsPageModel,
    | 'workspaces'
    | 'workspace'
    | 'workspaceId'
    | 'canManage'
    | 'serviceAccountsCount'
    | 'activeCount'
    | 'clientCount'
    | 'clearCreate'
    | 'setCreateOpen'
    | 'handleWorkspaceChange'
    | 'refetch'
  >,
) {
  const {
    workspaces,
    workspace,
    workspaceId,
    canManage,
    serviceAccountsCount,
    activeCount,
    clientCount,
    clearCreate,
    setCreateOpen,
    handleWorkspaceChange,
    refetch,
  } = props;

  return (
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
  );
}
