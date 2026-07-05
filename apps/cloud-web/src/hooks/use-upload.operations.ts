import { driveQueryKeys } from '../drive.queries';
import type { UploadFileInput } from '../drive.native-fs.service';
import { trackEvent } from '../drive.analytics';
import type { UploadProgress } from '../drive.uploads.types';
import { uploadTrackingPayload } from './use-upload.analytics';
import { initializeUploads } from './use-upload.shared';
import { performDirectoryUpload, performSingleUpload } from './use-upload.transfer';

export async function runDirectoryUpload({
  workspaceId,
  parentId,
  items,
  updateUpload,
  setUploads,
  acquireWakeLock,
  releaseWakeLock,
  invalidateFiles,
}: {
  workspaceId: string;
  parentId: string | undefined;
  items: UploadFileInput[];
  updateUpload: (key: string, partial: Partial<UploadProgress>) => void;
  setUploads: React.Dispatch<React.SetStateAction<Map<string, UploadProgress>>>;
  acquireWakeLock: () => Promise<void>;
  releaseWakeLock: () => Promise<void>;
  invalidateFiles: (queryKey: ReturnType<typeof driveQueryKeys.files>) => Promise<void>;
}) {
  setUploads((prev) => initializeUploads(prev, items));

  const itemsWithParent = items.map((item) => ({
    ...item,
    parentId: item.parentId ?? parentId,
  }));

  void acquireWakeLock();

  try {
    const totalBytes = itemsWithParent.reduce((acc, item) => acc + item.file.size, 0);
    trackEvent('file.upload_started', {
      workspace_id: workspaceId,
      file_count: itemsWithParent.length,
      ...uploadTrackingPayload(totalBytes),
    });

    const completedCount = await performDirectoryUpload(workspaceId, itemsWithParent, updateUpload);

    trackEvent('file.upload_completed', {
      workspace_id: workspaceId,
      file_count: completedCount,
      upload_count: itemsWithParent.length,
      ...uploadTrackingPayload(totalBytes),
      status:
        completedCount === itemsWithParent.length
          ? 'completed'
          : completedCount > 0
            ? 'partial'
            : 'failed',
    });
  } finally {
    void releaseWakeLock();
  }

  void invalidateFiles(driveQueryKeys.files(workspaceId));
}

export async function runSingleUpload({
  workspaceId,
  parentId,
  handle,
  updateUpload,
  setUploads,
  acquireWakeLock,
  releaseWakeLock,
  invalidateFiles,
}: {
  workspaceId: string;
  parentId: string | undefined;
  handle: FileSystemFileHandle;
  updateUpload: (key: string, partial: Partial<UploadProgress>) => void;
  setUploads: React.Dispatch<React.SetStateAction<Map<string, UploadProgress>>>;
  acquireWakeLock: () => Promise<void>;
  releaseWakeLock: () => Promise<void>;
  invalidateFiles: (queryKey: ReturnType<typeof driveQueryKeys.files>) => Promise<void>;
}) {
  const key = handle.name;
  const file = await handle.getFile();

  setUploads((prev) => {
    const next = new Map(prev);
    next.set(key, {
      fileName: key,
      fileSize: file.size,
      bytesUploaded: 0,
      status: 'pending',
    });
    return next;
  });

  void acquireWakeLock();

  try {
    trackEvent('file.upload_started', {
      workspace_id: workspaceId,
      file_count: 1,
      ...uploadTrackingPayload(file.size),
    });

    await performSingleUpload(workspaceId, handle, parentId, updateUpload);

    trackEvent('file.upload_completed', {
      workspace_id: workspaceId,
      file_count: 1,
      upload_count: 1,
      ...uploadTrackingPayload(file.size),
      status: 'completed',
    });
  } catch (error) {
    updateUpload(key, {
      status: 'error',
      error: error instanceof Error ? error.message : 'Upload failed',
    });

    trackEvent('file.upload_completed', {
      workspace_id: workspaceId,
      file_count: 0,
      upload_count: 1,
      ...uploadTrackingPayload(file.size),
      status: 'failed',
    });
  } finally {
    void releaseWakeLock();
  }

  void invalidateFiles(driveQueryKeys.files(workspaceId));
}
