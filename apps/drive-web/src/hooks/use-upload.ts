import { useCallback, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { driveQueryKeys } from '../drive.queries';
import { useWakeLock } from './use-wake-lock';
import {
  uploadDirectoryToDrive,
  uploadFileFromHandle,
  cancelFileUpload,
  type UploadFileInput,
} from '../drive.native-fs.service';
import { trackEvent } from '../drive.posthog';
import type { UploadProgress, UploadSummary } from '../drive.uploads.types';

function bytesBucket(bytes: number): string {
  if (bytes < 1024 * 1024) return '<1mb';
  if (bytes < 10 * 1024 * 1024) return '1-10mb';
  if (bytes < 100 * 1024 * 1024) return '10-100mb';
  if (bytes < 1024 * 1024 * 1024) return '100mb-1gb';
  return '1gb+';
}

export function useUpload(workspaceId: string, parentId?: string) {
  const queryClient = useQueryClient();
  const { acquire: acquireWakeLock, release: releaseWakeLock } = useWakeLock();
  const [uploads, setUploads] = useState<Map<string, UploadProgress>>(new Map());
  const abortRef = useRef<AbortController | null>(null);

  const uploadsArray = useCallback(() => Array.from(uploads.values()), [uploads]);

  const summary: UploadSummary = {
    totalFiles: uploads.size,
    totalBytes: Array.from(uploads.values()).reduce((acc, u) => acc + u.fileSize, 0),
    completedFiles: Array.from(uploads.values()).filter((u) => u.status === 'done').length,
    failedFiles: Array.from(uploads.values()).filter((u) => u.status === 'error').length,
  };

  const updateUpload = useCallback((key: string, partial: Partial<UploadProgress>) => {
    setUploads((prev) => {
      const next = new Map(prev);
      const existing = next.get(key);
      if (existing) {
        next.set(key, { ...existing, ...partial });
      }
      return next;
    });
  }, []);

  const uploadFiles = useCallback(
    async (items: UploadFileInput[]) => {
      abortRef.current = new AbortController();

      setUploads((prev) => {
        const next = new Map(prev);
        for (const item of items) {
          const key = item.name;
          if (!next.has(key)) {
            next.set(key, {
              fileName: item.name,
              fileSize: item.file.size,
              bytesUploaded: 0,
              status: 'pending',
            });
          }
        }
        return next;
      });

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
          total_bytes_bucket: bytesBucket(totalBytes),
        });

        let completedCount = 0;
        await uploadDirectoryToDrive(
          workspaceId,
          itemsWithParent,
          (fileName, progress) => {
            updateUpload(fileName, {
              bytesUploaded: progress.bytesUploaded,
              status: progress.phase === 'completing' ? 'completing' : 'uploading',
            });
          },
          (fileName, uploadId, error) => {
            updateUpload(fileName, {
              status: error ? 'error' : 'done',
              error,
              uploadId,
              bytesUploaded: uploads.get(fileName)?.fileSize ?? 0,
            });
            if (!error) completedCount += 1;
          },
        );
        trackEvent('file.upload_completed', {
          workspace_id: workspaceId,
          file_count: completedCount,
          upload_count: itemsWithParent.length,
          total_bytes_bucket: bytesBucket(totalBytes),
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

      void queryClient.invalidateQueries({ queryKey: driveQueryKeys.files(workspaceId) });
    },
    [workspaceId, parentId, queryClient, updateUpload],
  );

  const uploadSingleFile = useCallback(
    async (handle: FileSystemFileHandle) => {
      abortRef.current = new AbortController();

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
          total_bytes_bucket: bytesBucket(file.size),
        });

        const uploadId = await uploadFileFromHandle(workspaceId, handle, parentId, (progress) => {
          updateUpload(key, {
            bytesUploaded: progress.bytesUploaded,
            status: progress.phase === 'completing' ? 'completing' : 'uploading',
          });
        });

        updateUpload(key, {
          status: 'done',
          uploadId,
          bytesUploaded: file.size,
        });
        trackEvent('file.upload_completed', {
          workspace_id: workspaceId,
          file_count: 1,
          upload_count: 1,
          total_bytes_bucket: bytesBucket(file.size),
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
          total_bytes_bucket: bytesBucket(file.size),
          status: 'failed',
        });
      } finally {
        void releaseWakeLock();
      }

      void queryClient.invalidateQueries({ queryKey: driveQueryKeys.files(workspaceId) });
    },
    [workspaceId, parentId, queryClient, updateUpload],
  );

  const cancel = useCallback(async () => {
    abortRef.current?.abort();
    void releaseWakeLock();

    for (const [key, upload] of uploads) {
      if (upload.uploadId && (upload.status === 'uploading' || upload.status === 'pending')) {
        try {
          await cancelFileUpload(workspaceId, upload.uploadId);
        } catch {
          // Best-effort cancellation
        }
      }
      updateUpload(key, { status: 'error', error: 'Cancelled by user' });
    }
  }, [workspaceId, uploads, updateUpload]);

  const clearCompleted = useCallback(() => {
    setUploads((prev) => {
      const next = new Map(prev);
      for (const [key, upload] of next) {
        if (upload.status === 'done' || upload.status === 'error') {
          next.delete(key);
        }
      }
      return next;
    });
  }, []);

  return {
    uploads: uploadsArray,
    summary,
    uploadFiles,
    uploadSingleFile,
    cancel,
    clearCompleted,
    isUploading: Array.from(uploads.values()).some(
      (u) => u.status === 'uploading' || u.status === 'pending' || u.status === 'completing',
    ),
  };
}
