import { FileText, FolderOpen, Upload } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function DriveUploadDialogEmptyState({
  onUploadFiles,
  onUploadFolder,
}: {
  onUploadFiles: () => void;
  onUploadFolder: () => void;
}) {
  return (
    <div className="mt-8 flex flex-col items-center gap-3 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-muted">
        <Upload className="size-6 text-muted-foreground" />
      </div>
      <p className="text-sm text-muted-foreground">
        Ouvrez des fichiers ou un dossier depuis votre appareil pour les transferer vers le Drive.
      </p>
      <div className="mt-2 flex gap-2">
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
  );
}
