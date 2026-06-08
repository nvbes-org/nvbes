import { Button } from '@/components/ui/button';
import { DriveFilesGrid } from './DriveFilesGrid';
import { DriveFilesTable } from './DriveFilesTable';
import { DriveFilesToolbar } from './DriveFilesToolbar';
import { DriveEmptyState } from './DriveViewState';
import {
  createFolder,
  filterDriveEntries,
  selectEntry,
  toggleEntrySelection,
} from './drive.workspace.store';
import type { DriveSortKey, DriveViewMode, DriveWorkspaceState } from './drive.workspace.types';

type DriveFilesViewState = DriveWorkspaceState & { query?: string };

export function DriveFilesView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const filesState = state as DriveFilesViewState;
  const entries = filterDriveEntries(filesState.entries, {
    query: filesState.query,
    status: 'active',
    sort: filesState.sort,
  });

  function handleCreateFolder() {
    onStateChange(createFolder(filesState, 'Nouveau dossier'));
  }

  function handleViewModeChange(viewMode: DriveViewMode) {
    onStateChange({ ...filesState, viewMode });
  }

  function handleSortChange(sort: DriveSortKey) {
    onStateChange({ ...filesState, sort });
  }

  function handleOpenDetails(entryId: string) {
    onStateChange(selectEntry(filesState, entryId));
  }

  function handleToggleSelection(entryId: string) {
    onStateChange(toggleEntrySelection(filesState, entryId));
  }

  return (
    <section className="grid gap-4">
      <DriveFilesToolbar
        selectedCount={filesState.selectedEntryIds.length}
        viewMode={filesState.viewMode}
        sort={filesState.sort}
        onCreateFolder={handleCreateFolder}
        onViewModeChange={handleViewModeChange}
        onSortChange={handleSortChange}
      />
      {entries.length === 0 ? (
        <DriveEmptyState
          title="Aucun fichier actif"
          description="Importez un fichier ou creez un dossier pour demarrer cet espace Drive."
          action={
            <Button type="button" onClick={handleCreateFolder}>
              Nouveau dossier
            </Button>
          }
        />
      ) : filesState.viewMode === 'grid' ? (
        <DriveFilesGrid
          entries={entries}
          members={filesState.members}
          selectedEntryIds={filesState.selectedEntryIds}
          onOpenDetails={handleOpenDetails}
          onToggleSelection={handleToggleSelection}
        />
      ) : (
        <DriveFilesTable
          entries={entries}
          members={filesState.members}
          selectedEntryIds={filesState.selectedEntryIds}
          onOpenDetails={handleOpenDetails}
          onToggleSelection={handleToggleSelection}
        />
      )}
    </section>
  );
}
