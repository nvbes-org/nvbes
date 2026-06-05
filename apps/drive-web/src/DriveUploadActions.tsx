import { FolderOpen, Loader2, Upload } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { useFileSystemAccess } from './hooks/use-file-system-access';
import { useWakeLock } from './hooks/use-wake-lock';
import { DriveUploadDialog } from './DriveUploadDialog';
import type { UploadFileInput } from './drive.native-fs.service';
import type { UploadProgress, UploadSummary } from './drive.uploads.types';

type DriveUploadActionsProps = {
  workspaceId: string;
  parentId?: string;
};

export function DriveUploadActions({ workspaceId, parentId }: DriveUploadActionsProps) {
  const { supported, openFiles, openDirectory } = useFileSystemAccess();
  const { acquire: acquireWakeLock, release: releaseWakeLock } = useWakeLock();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [uploads, setUploads] = useState<UploadProgress[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  const summary: UploadSummary = {
    totalFiles: uploads.length,
    totalBytes: uploads.reduce((acc, u) => acc + u.fileSize, 0),
    completedFiles: uploads.filter((u) => u.status === 'done').length,
    failedFiles: uploads.filter((u) => u.status === 'error').length,
  };

  function updateUpload(fileName: string, partial: Partial<UploadProgress>) {
    setUploads((prev) => prev.map((u) => (u.fileName === fileName ? { ...u, ...partial } : u)));
  }

  function addUploads(items: UploadFileInput[]) {
    setUploads((prev) => {
      const existing = new Set(prev.map((u) => u.fileName));
      const newUploads = items
        .filter((item) => !existing.has(item.name))
        .map(
          (item): UploadProgress => ({
            fileName: item.name,
            fileSize: item.file.size,
            bytesUploaded: 0,
            status: 'pending',
          }),
        );
      return [...prev, ...newUploads];
    });
  }

  async function handleUploadFiles() {
    const results = await openFiles({ multiple: true });

    const items: UploadFileInput[] = results.map((r) => ({
      file: r.file,
      name: r.relativePath,
      mimeType: r.file.type || getMimeType(r.name),
      parentId,
    }));

    addUploads(items);
    setDialogOpen(true);
    setIsUploading(true);
    void acquireWakeLock();

    try {
      await startUploads(items);
    } finally {
      void releaseWakeLock();
    }
  }

  async function handleUploadFolder() {
    const results = await openDirectory();

    const items: UploadFileInput[] = results.map((r) => ({
      file: r.file,
      name: r.relativePath,
      mimeType: r.file.type || getMimeType(r.name),
      parentId,
    }));

    addUploads(items);
    setDialogOpen(true);
    setIsUploading(true);
    void acquireWakeLock();

    try {
      await startUploads(items);
    } finally {
      void releaseWakeLock();
    }
  }

  async function startUploads(items: UploadFileInput[]) {
    const { uploadFileToDrive } = await import('./drive.native-fs.service');

    await Promise.allSettled(
      items.map(async (item) => {
        try {
          const uploadId = await uploadFileToDrive(workspaceId, item, (progress) => {
            updateUpload(item.name, {
              bytesUploaded: progress.bytesUploaded,
              status: progress.phase === 'completing' ? 'completing' : 'uploading',
            });
          });
          updateUpload(item.name, { status: 'done', uploadId, bytesUploaded: item.file.size });
        } catch (error) {
          updateUpload(item.name, {
            status: 'error',
            error: error instanceof Error ? error.message : 'Upload failed',
          });
        }
      }),
    );

    setIsUploading(false);
  }

  async function handleCancel() {
    setIsUploading(false);
    void releaseWakeLock();
    setUploads((prev) =>
      prev.map((u) =>
        u.status === 'pending' || u.status === 'uploading' || u.status === 'completing'
          ? { ...u, status: 'error' as const, error: "Annule par l'utilisateur" }
          : u,
      ),
    );
  }

  function handleClear() {
    setUploads([]);
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
      />
    </>
  );
}

function getMimeType(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() ?? '';
  const mimeTypes: Record<string, string> = {
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    webp: 'image/webp',
    svg: 'image/svg+xml',
    pdf: 'application/pdf',
    doc: 'application/msword',
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    xls: 'application/vnd.ms-excel',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    zip: 'application/zip',
    tar: 'application/x-tar',
    gz: 'application/gzip',
    mp4: 'video/mp4',
    mp3: 'audio/mpeg',
    json: 'application/json',
    html: 'text/html',
    css: 'text/css',
    js: 'text/javascript',
    ts: 'text/typescript',
    md: 'text/markdown',
    txt: 'text/plain',
    csv: 'text/csv',
  };
  return mimeTypes[ext] ?? 'application/octet-stream';
}
