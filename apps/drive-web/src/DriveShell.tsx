import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { DriveAppLayout } from './DriveAppLayout';
import { DriveFilesView } from './DriveFilesView';
import { DriveEmptyState } from './DriveViewState';
import type { DriveMeResponse } from './drive.api';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import type { DriveEntry, DriveMember, DriveWorkspaceState } from './drive.workspace.types';
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
  const selectedEntry =
    workspace.detailsSelection?.type === 'entry'
      ? workspace.entries.find((entry) => entry.id === workspace.detailsSelection?.id)
      : null;

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
        selectedEntry ? (
          <DriveInterimDetails entry={selectedEntry} members={workspace.members} />
        ) : undefined
      }
      onModuleChange={handleModuleChange}
      onSectionChange={handleSectionChange}
      onQueryChange={(query) => setWorkspace((current) => ({ ...current, query }))}
    >
      {activeSection === 'files' ? (
        <DriveFilesView state={workspace} onStateChange={setWorkspace} />
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

function DriveInterimDetails({ entry, members }: { entry: DriveEntry; members: DriveMember[] }) {
  const owner = members.find((member) => member.id === entry.ownerId)?.name ?? 'Inconnu';

  return (
    <section className="grid gap-4">
      <div>
        <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Details provisoires
        </p>
        <h2 className="mt-1 break-words text-lg font-semibold">{entry.name}</h2>
      </div>
      <dl className="grid gap-3 text-sm">
        <div>
          <dt className="text-muted-foreground">Type</dt>
          <dd className="font-medium">{entry.kind}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Proprietaire</dt>
          <dd className="font-medium">{owner}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Statut</dt>
          <dd className="font-medium">{entry.shareStatus === 'shared' ? 'Partage' : 'Prive'}</dd>
        </div>
      </dl>
      <p className="text-xs text-muted-foreground">
        Le panneau de details complet arrive dans la prochaine tache.
      </p>
    </section>
  );
}
