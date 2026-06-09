import { useState } from 'react';
import { DriveAccountMenu } from './DriveAccountMenu';
import { DriveAppLayout } from './DriveAppLayout';
import { DriveDetailsPanel } from './DriveDetailsPanel';
import { DriveFilesView } from './DriveFilesView';
import { DriveSharedLinksView } from './DriveSharedLinksView';
import { DriveSharedWithMeView } from './DriveSharedWithMeView';
import { DriveStarredView } from './DriveStarredView';
import { DriveTrashView } from './DriveTrashView';
import { DriveWorkspaceSwitcher } from './DriveWorkspaceSwitcher';
import type { DriveMeResponse } from './drive.api';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import type { DriveWorkspaceState } from './drive.workspace.types';

const MOCK_WORKSPACE_NAME = 'Workspace personnel';

function labelForModule(moduleId: DriveWorkspaceState['activeModuleId']): string {
  if (moduleId === 'sharing') return 'Liens partages';
  if (moduleId === 'shared-with-me') return 'Partages avec moi';
  if (moduleId === 'starred') return 'Suivis';
  if (moduleId === 'trash') return 'Corbeille';
  return 'Fichiers';
}

export function DriveShell({ accessToken, me }: { accessToken: string; me: DriveMeResponse }) {
  const [workspace, setWorkspace] = useState<DriveWorkspaceState>(() =>
    createInitialDriveWorkspace(),
  );
  const currentWorkspace =
    me.workspaces.find((workspace) => workspace.id === me.current_workspace_id) ?? me.workspaces[0];
  const workspaceName = currentWorkspace?.name ?? MOCK_WORKSPACE_NAME;

  function handleModuleChange(moduleId: DriveWorkspaceState['activeModuleId']) {
    setWorkspace((current) => ({
      ...current,
      activeModuleId: moduleId,
      detailsSelection: null,
      selectedEntryIds: [],
    }));
  }

  return (
    <DriveAppLayout
      activeModule={workspace.activeModuleId}
      workspaceName={workspaceName}
      sectionLabel={labelForModule(workspace.activeModuleId)}
      query={workspace.query}
      billing={workspace.billing}
      topbarActions={<DriveAccountMenu accessToken={accessToken} user={me.user} />}
      workspaceSwitcher={
        <DriveWorkspaceSwitcher
          accessToken={accessToken}
          workspaceName={workspaceName}
          workspaceId={me.current_workspace_id}
          workspaces={me.workspaces}
        />
      }
      details={
        workspace.detailsSelection ? (
          <DriveDetailsPanel
            state={workspace}
            onClose={() => setWorkspace((current) => ({ ...current, detailsSelection: null }))}
          />
        ) : undefined
      }
      onModuleChange={handleModuleChange}
      onQueryChange={(query) => setWorkspace((current) => ({ ...current, query }))}
    >
      {workspace.activeModuleId === 'trash' ? (
        <DriveTrashView state={workspace} onStateChange={setWorkspace} />
      ) : workspace.activeModuleId === 'sharing' ? (
        <DriveSharedLinksView state={workspace} onStateChange={setWorkspace} />
      ) : workspace.activeModuleId === 'shared-with-me' ? (
        <DriveSharedWithMeView state={workspace} onStateChange={setWorkspace} />
      ) : workspace.activeModuleId === 'starred' ? (
        <DriveStarredView state={workspace} onStateChange={setWorkspace} />
      ) : (
        <DriveFilesView
          workspaceId={me.current_workspace_id ?? currentWorkspace?.id ?? 'demo-workspace'}
          state={workspace}
          onStateChange={setWorkspace}
        />
      )}
    </DriveAppLayout>
  );
}
