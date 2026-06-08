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

export function DriveFilesView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const entries = filterDriveEntries(state.entries, {
    query: state.query,
    status: 'active',
    sort: state.sort,
  });

  function handleCreateFolder() {
    onStateChange(createFolder(state, 'Nouveau dossier'));
  }

  function handleViewModeChange(viewMode: DriveViewMode) {
    onStateChange({ ...state, viewMode });
  }

  function handleSortChange(sort: DriveSortKey) {
    onStateChange({ ...state, sort });
  }

  function handleOpenDetails(entryId: string) {
    onStateChange(selectEntry(state, entryId));
  }

  function handleToggleSelection(entryId: string) {
    onStateChange(toggleEntrySelection(state, entryId));
  }

  return (
    <section className="grid gap-4">
      <DriveFilesToolbar
        selectedCount={state.selectedEntryIds.length}
        viewMode={state.viewMode}
        sort={state.sort}
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
      ) : state.viewMode === 'grid' ? (
        <DriveFilesGrid
          entries={entries}
          members={state.members}
          selectedEntryIds={state.selectedEntryIds}
          onOpenDetails={handleOpenDetails}
          onToggleSelection={handleToggleSelection}
        />
      ) : (
        <DriveFilesTable
          entries={entries}
          members={state.members}
          selectedEntryIds={state.selectedEntryIds}
          onOpenDetails={handleOpenDetails}
          onToggleSelection={handleToggleSelection}
        />
      )}
    </section>
  );
}
