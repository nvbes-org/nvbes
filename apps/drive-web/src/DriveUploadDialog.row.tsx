import { CheckCircle2, Loader2, XCircle } from 'lucide-react';
import { Progress } from '@/components/ui/progress';
import type { UploadProgress } from './drive.uploads.types';
import { formatUploadSize } from './DriveUploadDialog.utils';

export function UploadItemRow({ item }: { item: UploadProgress }) {
  const progressPct =
    item.fileSize > 0 ? Math.min(100, Math.round((item.bytesUploaded / item.fileSize) * 100)) : 0;

  return (
    <div className="rounded-xl border border-border/60 bg-background p-3 shadow-sm">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="flex min-w-0 items-start gap-2.5">
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
                `${formatUploadSize(item.bytesUploaded)} / ${formatUploadSize(item.fileSize)}`}
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
