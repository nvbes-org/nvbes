import { FolderOpen, Loader2, Upload } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { toDroppedUploadFileInputs, toUploadFileInputs } from './drive.uploads.prepare';
import { useFileSystemAccess } from './hooks/use-file-system-access';
import { useUpload } from './hooks/use-upload';
import { DriveUploadDialog } from './DriveUploadDialog';

type DriveUploadActionsProps = {
  workspaceId: string;
  parentId?: string;
};

export function DriveUploadActions({ workspaceId, parentId }: DriveUploadActionsProps) {
  const { supported, openFiles, openDirectory } = useFileSystemAccess();
  const { uploads, summary, uploadFiles, cancel, clearCompleted, isUploading } = useUpload(
    workspaceId,
    parentId,
  );
  const [dialogOpen, setDialogOpen] = useState(false);

  async function handleUploadFiles() {
    const results = await openFiles({ multiple: true });
    const items = toUploadFileInputs(results, parentId);
    setDialogOpen(true);
    await uploadFiles(items);
  }

  async function handleUploadFolder() {
    const results = await openDirectory();
    const items = toUploadFileInputs(results, parentId);
    setDialogOpen(true);
    await uploadFiles(items);
  }

  async function handleUploadDroppedFiles(files: File[]) {
    const items = toDroppedUploadFileInputs(files, parentId);
    setDialogOpen(true);
    await uploadFiles(items);
  }

  async function handleCancel() {
    await cancel();
  }

  function handleClear() {
    clearCompleted();
  }

  if (!supported) {
    return null;
  }

  return (
    <>
      <div className="flex items-center gap-2">
        <Button variant="outline" className="h-10 px-4" onClick={handleUploadFiles}>
          <Upload data-icon="inline-start" />
          Ouvrir
        </Button>
        <Button
          variant="ghost"
          className="h-10 px-3"
          onClick={handleUploadFolder}
          aria-label="Ouvrir un dossier"
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
            {uploads.some((u) => u.status === 'uploading' || u.status === 'pending') && (
              <span className="absolute -right-0.5 -top-0.5 flex size-4 items-center justify-center rounded-full bg-primary text-[10px] font-bold text-primary-foreground">
                {uploads.filter((u) => u.status === 'pending' || u.status === 'uploading').length}
              </span>
            )}
          </Button>
        )}
      </div>

      <DriveUploadDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        uploads={uploads}
        summary={summary}
        isUploading={isUploading}
        onCancel={handleCancel}
        onClear={handleClear}
        onUploadFiles={handleUploadFiles}
        onUploadFolder={handleUploadFolder}
        onUploadDroppedFiles={handleUploadDroppedFiles}
      />
    </>
  );
}
