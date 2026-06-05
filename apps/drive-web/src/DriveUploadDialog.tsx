import { CheckCircle2, FileText, FolderOpen, Loader2, Upload, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import type { UploadProgress, UploadSummary } from './drive.uploads.types';

type DriveUploadDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  uploads: UploadProgress[];
  summary: UploadSummary;
  isUploading: boolean;
  onCancel: () => void;
  onClear: () => void;
  onUploadFiles: () => void;
  onUploadFolder: () => void;
};

export function DriveUploadDialog({
  open,
  onOpenChange,
  uploads,
  summary,
  isUploading,
  onCancel,
  onClear,
  onUploadFiles,
  onUploadFolder,
}: DriveUploadDialogProps) {
  const hasCompleted = summary.completedFiles > 0 || summary.failedFiles > 0;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-96 sm:max-w-md">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <Upload className="size-4" />
            Transferts
          </SheetTitle>
          <SheetDescription>
            {summary.totalFiles === 0
              ? 'Aucun transfert en cours'
              : `${summary.completedFiles + summary.failedFiles}/${summary.totalFiles} fichiers`}
          </SheetDescription>
        </SheetHeader>

        {summary.totalFiles === 0 ? (
          <div className="mt-8 flex flex-col items-center gap-3 text-center">
            <div className="flex size-12 items-center justify-center rounded-full bg-muted">
              <Upload className="size-6 text-muted-foreground" />
            </div>
            <p className="text-sm text-muted-foreground">
              Ouvrez des fichiers ou un dossier depuis votre appareil pour les transferer vers le
              Drive.
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
        ) : (
          <div className="mt-4 space-y-3">
            {uploads.map((item) => (
              <UploadItemRow key={item.fileName} item={item} />
            ))}

            <div className="border-t border-border/60 pt-3">
              {isUploading ? (
                <Button variant="secondary" className="w-full" onClick={onCancel}>
                  <XCircle data-icon="inline-start" />
                  Annuler tout
                </Button>
              ) : hasCompleted ? (
                <Button variant="secondary" className="w-full" onClick={onClear}>
                  Effacer la liste
                </Button>
              ) : null}
            </div>
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}

function UploadItemRow({ item }: { item: UploadProgress }) {
  const progressPct =
    item.fileSize > 0 ? Math.min(100, Math.round((item.bytesUploaded / item.fileSize) * 100)) : 0;

  return (
    <div className="rounded-xl border border-border/60 bg-background p-3 shadow-sm">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="flex items-start gap-2.5 min-w-0">
          <div className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-lg bg-muted">
            {item.status === 'done' ? (
              <CheckCircle2 className="size-3.5 text-emerald-500" />
            ) : item.status === 'error' ? (
              <XCircle className="size-3.5 text-destructive" />
            ) : (
              <Loader2 className="size-3.5 animate-spin text-muted-foreground" />
            )}
          </div>
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{item.fileName}</p>
            <p className="text-xs text-muted-foreground">
              {item.status === 'pending' && 'En attente...'}
              {item.status === 'uploading' &&
                `${formatSize(item.bytesUploaded)} / ${formatSize(item.fileSize)}`}
              {item.status === 'completing' && 'Finalisation...'}
              {item.status === 'done' && 'Termine'}
              {item.status === 'error' && (item.error ?? 'Erreur')}
            </p>
          </div>
        </div>
      </div>
      {item.status === 'uploading' && (
        <Progress value={progressPct} className="h-1.5" aria-label={`Upload: ${progressPct}%`} />
      )}
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 o';
  const units = ['o', 'Ko', 'Mo', 'Go', 'To'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[i]}`;
}
