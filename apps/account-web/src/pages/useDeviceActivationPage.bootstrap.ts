import type { AccountWorkspace } from '@nvbes/identity-client';

export async function loadDeviceActivationWorkspaces({
  currentWorkspaceId,
  listWorkspaces,
  setSelectedWorkspace,
  setWorkspaces,
}: {
  currentWorkspaceId: string | null | undefined;
  listWorkspaces: () => Promise<AccountWorkspace[]>;
  setSelectedWorkspace: (value: string) => void;
  setWorkspaces: (value: AccountWorkspace[]) => void;
}) {
  const workspaceList = await listWorkspaces();
  setWorkspaces(workspaceList);

  if (currentWorkspaceId) {
    setSelectedWorkspace(currentWorkspaceId);
    return;
  }

  if (workspaceList.length > 0) {
    setSelectedWorkspace(workspaceList[0].id);
  }
}
