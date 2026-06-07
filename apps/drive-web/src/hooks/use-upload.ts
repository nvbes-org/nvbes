import { useCallback, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWakeLock } from './use-wake-lock';
import type { UploadFileInput } from '../drive.native-fs.service';
import type { UploadProgress, UploadSummary } from '../drive.uploads.types';
import { runDirectoryUpload, runSingleUpload } from './use-upload.operations';
import {
  buildUploadSummary,
  clearFinishedUploads,
  isActiveUploadStatus,
} from './use-upload.shared';
import { cancelUploads } from './use-upload.transfer';

export function useUpload(workspaceId: string, parentId?: string) {
  const queryClient = useQueryClient();
  const { acquire: acquireWakeLock, release: releaseWakeLock } = useWakeLock();
  const [uploads, setUploads] = useState<Map<string, UploadProgress>>(new Map());
  const abortRef = useRef<AbortController | null>(null);

  const uploadsList = useMemo(() => Array.from(uploads.values()), [uploads]);

  const summary: UploadSummary = buildUploadSummary(uploadsList);

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
      await runDirectoryUpload({
        workspaceId,
        parentId,
        items,
        updateUpload,
        setUploads,
        acquireWakeLock,
        releaseWakeLock,
        invalidateFiles: async (queryKey) =>
          queryClient.invalidateQueries({
            queryKey,
          }),
      });
    },
    [workspaceId, parentId, queryClient, updateUpload],
  );

  const uploadSingleFile = useCallback(
    async (handle: FileSystemFileHandle) => {
      abortRef.current = new AbortController();
      await runSingleUpload({
        workspaceId,
        parentId,
        handle,
        updateUpload,
        setUploads,
        acquireWakeLock,
        releaseWakeLock,
        invalidateFiles: async (queryKey) =>
          queryClient.invalidateQueries({
            queryKey,
          }),
      });
    },
    [workspaceId, parentId, queryClient, updateUpload],
  );

  const cancel = useCallback(async () => {
    abortRef.current?.abort();
    void releaseWakeLock();
    await cancelUploads(workspaceId, uploads, updateUpload);
  }, [workspaceId, uploads, updateUpload]);

  const clearCompleted = useCallback(() => {
    setUploads((prev) => clearFinishedUploads(prev));
  }, []);

  return {
    uploads: uploadsList,
    summary,
    uploadFiles,
    uploadSingleFile,
    cancel,
    clearCompleted,
    isUploading: uploadsList.some((upload) => isActiveUploadStatus(upload.status)),
  };
}
