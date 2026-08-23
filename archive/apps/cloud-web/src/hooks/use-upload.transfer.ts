import {
  cancelFileUpload,
  uploadDirectoryToDrive,
  uploadFileFromHandle,
  type UploadFileInput,
} from '../drive.native-fs.service';
import type { UploadProgress } from '../drive.uploads.types';

type UpdateUpload = (key: string, partial: Partial<UploadProgress>) => void;

export async function performDirectoryUpload(
  workspaceId: string,
  items: UploadFileInput[],
  updateUpload: UpdateUpload,
): Promise<number> {
  let completedCount = 0;
  const fileSizeByName = new Map(items.map((item) => [item.name, item.file.size]));

  await uploadDirectoryToDrive(
    workspaceId,
    items,
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
        bytesUploaded: fileSizeByName.get(fileName) ?? 0,
      });
      if (!error) completedCount += 1;
    },
  );

  return completedCount;
}

export async function performSingleUpload(
  workspaceId: string,
  handle: FileSystemFileHandle,
  parentId: string | undefined,
  updateUpload: UpdateUpload,
): Promise<{ file: File; uploadId: string }> {
  const key = handle.name;
  const file = await handle.getFile();
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

  return { file, uploadId };
}

export async function cancelUploads(
  workspaceId: string,
  uploads: Map<string, UploadProgress>,
  updateUpload: UpdateUpload,
): Promise<void> {
  for (const [key, upload] of uploads) {
    if (upload.uploadId && (upload.status === 'uploading' || upload.status === 'pending')) {
      try {
        await cancelFileUpload(workspaceId, upload.uploadId);
      } catch {
        // Best-effort cancellation
      }
    }
    updateUpload(key, { status: 'error', error: "Annule par l'utilisateur" });
  }
}
