import { Upload, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { DriveUploadDialogEmptyState } from './DriveUploadDialog.empty';
import { UploadItemRow } from './DriveUploadDialog.row';
import type { DriveUploadDialogProps } from './DriveUploadDialog.types';

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
          <DriveUploadDialogEmptyState
            onUploadFiles={onUploadFiles}
            onUploadFolder={onUploadFolder}
          />
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
