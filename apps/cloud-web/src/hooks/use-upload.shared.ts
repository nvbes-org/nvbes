import type { UploadProgress, UploadSummary } from '../drive.uploads.types';

export function isActiveUploadStatus(status: UploadProgress['status']): boolean {
  return status === 'pending' || status === 'uploading' || status === 'completing';
}

export function buildUploadSummary(uploads: UploadProgress[]): UploadSummary {
  return {
    totalFiles: uploads.length,
    totalBytes: uploads.reduce((acc, upload) => acc + upload.fileSize, 0),
    completedFiles: uploads.filter((upload) => upload.status === 'done').length,
    failedFiles: uploads.filter((upload) => upload.status === 'error').length,
  };
}

export function initializeUploads(
  previous: Map<string, UploadProgress>,
  items: Array<{ name: string; file: File }>,
): Map<string, UploadProgress> {
  const next = new Map(previous);
  for (const item of items) {
    if (!next.has(item.name)) {
      next.set(item.name, {
        fileName: item.name,
        fileSize: item.file.size,
        bytesUploaded: 0,
        status: 'pending',
      });
    }
  }
  return next;
}

export function clearFinishedUploads(
  previous: Map<string, UploadProgress>,
): Map<string, UploadProgress> {
  const next = new Map(previous);
  for (const [key, upload] of next) {
    if (upload.status === 'done' || upload.status === 'error') {
      next.delete(key);
    }
  }
  return next;
}
