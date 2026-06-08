import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { DriveAppLayout } from './DriveAppLayout';
import { DriveDetailsPanel } from './DriveDetailsPanel';
import { DriveFilesView } from './DriveFilesView';
import { DriveSharedLinksView } from './DriveSharedLinksView';
import { DriveEmptyState } from './DriveViewState';
import { DriveTrashView } from './DriveTrashView';
import type { DriveMeResponse } from './drive.api';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import type { DriveWorkspaceState } from './drive.workspace.types';
import {
  firstSectionForModule,
  labelForSection,
  type DriveModuleId,
  type DriveSectionId,
} from './DriveSectionNav';

const MOCK_WORKSPACE_NAME = 'Workspace personnel';

export function DriveShell({ accessToken, me }: { accessToken: string; me: DriveMeResponse }) {
  const [workspace, setWorkspace] = useState<DriveWorkspaceState>(() => createInitialDriveWorkspace());
  const [activeModule, setActiveModule] = useState<DriveModuleId>('drive');
  const [activeSection, setActiveSection] = useState<DriveSectionId>(() => firstSectionForModule('drive'));
  const currentWorkspace =
    me.workspaces.find((workspace) => workspace.id === me.current_workspace_id) ?? me.workspaces[0];
  const workspaceName = currentWorkspace?.name ?? MOCK_WORKSPACE_NAME;

  function handleModuleChange(moduleId: DriveModuleId) {
    setActiveModule(moduleId);
    setActiveSection(firstSectionForModule(moduleId));
    setWorkspace((current) => ({
      ...current,
      detailsSelection: null,
      selectedEntryIds: [],
    }));
  }

  function handleSectionChange(sectionId: DriveSectionId) {
    setActiveSection(sectionId);
    setWorkspace((current) => ({
      ...current,
      detailsSelection: null,
      selectedEntryIds: [],
    }));
  }

  return (
    <DriveAppLayout
      activeModule={activeModule}
      activeSection={activeSection}
      workspaceName={workspaceName}
      sectionLabel={labelForSection(activeSection)}
      query={workspace.query}
      billing={workspace.billing}
      toast={workspace.toast}
      details={
        workspace.detailsSelection ? (
          <DriveDetailsPanel
            state={workspace}
            onClose={() => setWorkspace((current) => ({ ...current, detailsSelection: null }))}
          />
        ) : undefined
      }
      onModuleChange={handleModuleChange}
      onSectionChange={handleSectionChange}
      onQueryChange={(query) => setWorkspace((current) => ({ ...current, query }))}
    >
      {activeSection === 'files' ? (
        <DriveFilesView state={workspace} onStateChange={setWorkspace} />
      ) : activeSection === 'shared-links' ? (
        <DriveSharedLinksView state={workspace} onStateChange={setWorkspace} />
      ) : activeSection === 'trash' ? (
        <DriveTrashView state={workspace} onStateChange={setWorkspace} />
      ) : (
        <DriveEmptyState
          title={`${labelForSection(activeSection)} arrive bientot`}
          description="La navigation applicative est en place. Les vues metier seront connectees aux donnees Drive dans les prochaines taches du redesign."
          action={
            <Button type="button" variant="outline" data-session-state={accessToken.length > 0 ? 'active' : 'missing'}>
              Session {me.user.email}
            </Button>
          }
        />
      )}
    </DriveAppLayout>
  );
}
