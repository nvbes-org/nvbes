import type { SignedUploadUrl } from './drive.uploads.api';
import { createUpload, completeUpload, cancelUpload } from './drive.uploads.api';

export type UploadFileInput = {
  file: File;
  name: string;
  mimeType: string;
  parentId?: string;
};

export type UploadProgressCallback = (progress: {
  bytesUploaded: number;
  totalBytes: number;
  phase: 'uploading' | 'completing';
}) => void;

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

  const failures = results.filter((r): r is PromiseRejectedResult => r.status === 'rejected');
  if (failures.length > 0) {
    const messages = failures
      .map((f) => (f.reason instanceof Error ? f.reason.message : 'Unknown error'))
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

export async function computeChecksum(blob: Blob): Promise<string> {
  const buffer = await blob.arrayBuffer();
  const hash = await crypto.subtle.digest('SHA-256', buffer);
  const bytes = new Uint8Array(hash);
  let hex = '';
  for (const byte of bytes) {
    hex += byte.toString(16).padStart(2, '0');
  }
  return hex;
}

async function compressFile(file: File): Promise<Blob> {
  const stream = file.stream().pipeThrough(new CompressionStream('gzip'));
  return new Response(stream).blob();
}

function guessMimeType(fileName: string): string {
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

async function uploadToPresignedUrl(
  uploadUrl: SignedUploadUrl,
  blob: Blob,
  onProgress?: UploadProgressCallback,
): Promise<void> {
  const { url, method, required_headers } = uploadUrl;

  const headers: Record<string, string> = {};
  for (const h of required_headers) {
    headers[h.name] = h.value;
  }

  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open(method, url, true);

    for (const [key, value] of Object.entries(headers)) {
      xhr.setRequestHeader(key, value);
    }

    xhr.upload.onprogress = (event) => {
      if (event.lengthComputable) {
        onProgress?.({
          bytesUploaded: event.loaded,
          totalBytes: event.total,
          phase: 'uploading',
        });
      }
    };

    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        resolve();
      } else {
        reject(new Error(`Upload failed with status ${xhr.status}: ${xhr.statusText}`));
      }
    };

    xhr.onerror = () => reject(new Error('Upload failed: network error'));
    xhr.onabort = () => reject(new Error('Upload cancelled'));

    xhr.send(blob);
  });
}

export async function cancelFileUpload(workspaceId: string, uploadId: string): Promise<void> {
  await cancelUpload(workspaceId, uploadId);
}
