import { cancelUpload, completeUpload, createUpload } from './drive.uploads.api';
import type { UploadFileInput, UploadProgressCallback } from './drive.native-fs.types';
import { uploadToPresignedUrl } from './drive.native-fs.transport';
import { compressFile, computeChecksum, guessMimeType } from './drive.native-fs.utils';

export async function uploadFileToDrive(
  workspaceId: string,
  input: UploadFileInput,
  onProgress?: UploadProgressCallback,
): Promise<string> {
  const compressed = await compressFile(input.file);
  const compressedChecksum = await computeChecksum(compressed);

  const response = await createUpload(workspaceId, {
    parentId: input.parentId,
    name: input.name,
    mimeType: input.mimeType,
    expectedSizeBytes: compressed.size,
    expectedChecksum: compressedChecksum,
  });

  await uploadToPresignedUrl(response.upload_url, compressed, onProgress);

  onProgress?.({
    bytesUploaded: compressed.size,
    totalBytes: compressed.size,
    phase: 'completing',
  });

  await completeUpload(workspaceId, response.upload_id, {
    sizeBytes: compressed.size,
    checksum: compressedChecksum,
  });

  return response.upload_id;
}

export async function uploadDirectoryToDrive(
  workspaceId: string,
  files: UploadFileInput[],
  onFileProgress?: (
    fileName: string,
    progress: {
      bytesUploaded: number;
      totalBytes: number;
      phase: 'uploading' | 'completing';
    },
  ) => void,
  onFileComplete?: (fileName: string, uploadId?: string, error?: string) => void,
): Promise<void> {
  const results = await Promise.allSettled(
    files.map(async (input) => {
      try {
        const uploadId = await uploadFileToDrive(workspaceId, input, (progress) => {
          onFileProgress?.(input.name, progress);
        });
        onFileComplete?.(input.name, uploadId);
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Upload failed';
        onFileComplete?.(input.name, undefined, message);
      }
    }),
  );

  const failures = results.filter(
    (result): result is PromiseRejectedResult => result.status === 'rejected',
  );
  if (failures.length > 0) {
    const messages = failures
      .map((failure) =>
        failure.reason instanceof Error ? failure.reason.message : 'Unknown error',
      )
      .join('; ');
    throw new Error(`Upload completed with ${failures.length} failure(s): ${messages}`);
  }
}

export async function uploadFileFromHandle(
  workspaceId: string,
  handle: FileSystemFileHandle,
  parentId?: string,
  onProgress?: UploadProgressCallback,
): Promise<string> {
  const file = await handle.getFile();
  return uploadFileToDrive(
    workspaceId,
    {
      file,
      name: handle.name,
      mimeType: file.type || guessMimeType(handle.name),
      parentId,
    },
    onProgress,
  );
}

export async function cancelFileUpload(workspaceId: string, uploadId: string): Promise<void> {
  await cancelUpload(workspaceId, uploadId);
}
