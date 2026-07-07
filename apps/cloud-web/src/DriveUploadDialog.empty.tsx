import { FileText, FolderOpen, Upload } from 'lucide-react';
import type { UploadHookControl } from '@better-upload/client';
import { Button } from '@/components/ui/button';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { UploadDropzone } from '@/components/ui/upload-dropzone';

export function DriveUploadDialogEmptyState({
  onUploadFiles,
  onUploadFolder,
  onUploadDroppedFiles,
  isUploading,
}: {
  onUploadFiles: () => void;
  onUploadFolder: () => void;
  onUploadDroppedFiles: (files: File[]) => void | Promise<void>;
  isUploading: boolean;
}) {
  const dropzoneControl = {
    isPending: isUploading,
    upload: () => undefined,
  } as unknown as UploadHookControl<true>;

  return (
    <Empty className="mt-8 border">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <Upload />
        </EmptyMedia>
        <EmptyTitle>Aucun transfert en attente</EmptyTitle>
        <EmptyDescription>
          Ouvrez des fichiers ou un dossier depuis votre appareil pour les transferer vers le Drive.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <div className="flex w-full flex-col gap-3">
          <UploadDropzone
            control={dropzoneControl}
            description="Deposez plusieurs fichiers pour les envoyer dans le dossier courant."
            uploadOverride={(files) => {
              void onUploadDroppedFiles(Array.from(files));
            }}
          />
          <div className="flex justify-center gap-2">
            <Button variant="outline" onClick={onUploadFiles}>
              <FileText data-icon="inline-start" />
              Fichiers
            </Button>
            <Button variant="outline" onClick={onUploadFolder}>
              <FolderOpen data-icon="inline-start" />
              Dossier
            </Button>
          </div>
        </div>
      </EmptyContent>
    </Empty>
  );
}
