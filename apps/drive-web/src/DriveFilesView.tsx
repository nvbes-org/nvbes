import { FolderOpen, Loader2, Upload } from 'lucide-react';
import { useState } from 'react';
import type { DragEvent } from 'react';
import { Button } from '@/components/ui/button';
import { DriveUploadDialog } from './DriveUploadDialog';
import { DriveFilesGrid } from './DriveFilesGrid';
import { DriveFilesTable } from './DriveFilesTable';
import { DriveFilesToolbar } from './DriveFilesToolbar';
import { DriveEmptyState } from './DriveViewState';
import { toDroppedUploadFileInputs, toUploadFileInputs } from './drive.uploads.prepare';
import {
  createFolder,
  filterDriveEntries,
  selectEntry,
  selectEntryFromSecondaryAction,
  toggleEntrySelection,
} from './drive.workspace.store';
import type { DriveSortKey, DriveViewMode, DriveWorkspaceState } from './drive.workspace.types';
import { useFileSystemAccess } from './hooks/use-file-system-access';
import { useUpload } from './hooks/use-upload';

export function DriveFilesView({
  workspaceId,
  state,
  onStateChange,
}: {
  workspaceId: string;
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const { supported, openFiles, openDirectory } = useFileSystemAccess();
  const { uploads, summary, uploadFiles, cancel, clearCompleted, isUploading } = useUpload(
    workspaceId,
    state.currentFolderId ?? undefined,
  );
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dragDepth, setDragDepth] = useState(0);
  const entries = filterDriveEntries(state.entries, {
    query: state.query,
    status: 'active',
    sort: state.sort,
  });
  const isDraggingFiles = dragDepth > 0;

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

  function handleSecondarySelection(entryId: string) {
    onStateChange(selectEntryFromSecondaryAction(state, entryId));
  }

  async function handleUploadFiles() {
    const results = await openFiles({ multiple: true });
    const items = toUploadFileInputs(results, state.currentFolderId ?? undefined);
    if (items.length === 0) return;
    setDialogOpen(true);
    await uploadFiles(items);
  }

  async function handleUploadFolder() {
    const results = await openDirectory();
    const items = toUploadFileInputs(results, state.currentFolderId ?? undefined);
    if (items.length === 0) return;
    setDialogOpen(true);
    await uploadFiles(items);
  }

  async function handleDroppedFiles(files: Iterable<File>) {
    const items = toDroppedUploadFileInputs(files, state.currentFolderId ?? undefined);
    if (items.length === 0) return;
    setDialogOpen(true);
    await uploadFiles(items);
  }

  function handleDragEnter(event: DragEvent<HTMLElement>) {
    if (!event.dataTransfer.types.includes('Files')) return;
    event.preventDefault();
    setDragDepth((value) => value + 1);
  }

  function handleDragOver(event: DragEvent<HTMLElement>) {
    if (!event.dataTransfer.types.includes('Files')) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'copy';
  }

  function handleDragLeave(event: DragEvent<HTMLElement>) {
    if (!event.dataTransfer.types.includes('Files')) return;
    event.preventDefault();
    if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
    setDragDepth(0);
  }

  async function handleDrop(event: DragEvent<HTMLElement>) {
    if (!event.dataTransfer.files.length) return;
    event.preventDefault();
    setDragDepth(0);
    await handleDroppedFiles(event.dataTransfer.files);
  }

  return (
    <>
      <section
        className="relative grid gap-4"
        onDragEnter={handleDragEnter}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={(event) => {
          void handleDrop(event);
        }}
      >
        <DriveFilesToolbar
          selectedCount={state.selectedEntryIds.length}
          viewMode={state.viewMode}
          sort={state.sort}
          uploadActions={
            supported ? (
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  className="h-10 px-4"
                  onClick={() => void handleUploadFiles()}
                >
                  <Upload data-icon="inline-start" />
                  Importer
                </Button>
                <Button
                  variant="ghost"
                  className="h-10 px-3"
                  onClick={() => void handleUploadFolder()}
                  aria-label="Importer un dossier"
                >
                  <FolderOpen data-icon="inline-start" />
                </Button>
                {uploads.length > 0 && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="relative size-10"
                    onClick={() => setDialogOpen(true)}
                    aria-label="Voir les transferts"
                  >
                    {isUploading ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <Upload className="size-4" />
                    )}
                  </Button>
                )}
              </div>
            ) : undefined
          }
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
            onSecondarySelect={handleSecondarySelection}
            onToggleSelection={handleToggleSelection}
          />
        ) : (
          <DriveFilesTable
            entries={entries}
            members={state.members}
            selectedEntryIds={state.selectedEntryIds}
            onOpenDetails={handleOpenDetails}
            onSecondarySelect={handleSecondarySelection}
            onToggleSelection={handleToggleSelection}
          />
        )}
        {isDraggingFiles && (
          <div className="pointer-events-none absolute inset-0 z-10 rounded-[1.75rem] border-2 border-dashed border-primary/60 bg-primary/8 p-4">
            <div className="grid h-full place-items-center rounded-[1.25rem] bg-background/88 backdrop-blur-sm">
              <div className="grid justify-items-center gap-2 text-center">
                <Upload className="size-8 text-primary" aria-hidden="true" />
                <p className="text-base font-medium">Deposez vos fichiers pour les importer</p>
                <p className="text-sm text-muted-foreground">
                  Les fichiers seront ajoutes dans le dossier courant.
                </p>
              </div>
            </div>
          </div>
        )}
      </section>
      <DriveUploadDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        uploads={uploads}
        summary={summary}
        isUploading={isUploading}
        onCancel={() => void cancel()}
        onClear={clearCompleted}
        onUploadFiles={() => void handleUploadFiles()}
        onUploadFolder={() => void handleUploadFolder()}
        onUploadDroppedFiles={(files) => void handleDroppedFiles(files)}
      />
    </>
  );
}
