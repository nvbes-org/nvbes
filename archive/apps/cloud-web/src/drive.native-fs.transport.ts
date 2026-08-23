import type { SignedUploadUrl } from './drive.uploads.api';
import type { UploadProgressCallback } from './drive.native-fs.types';

export async function uploadToPresignedUrl(
  uploadUrl: SignedUploadUrl,
  blob: Blob,
  onProgress?: UploadProgressCallback,
): Promise<void> {
  const { url, method, required_headers } = uploadUrl;

  const headers: Record<string, string> = {};
  for (const header of required_headers) {
    headers[header.name] = header.value;
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
