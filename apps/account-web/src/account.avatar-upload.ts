import { createTrustedWorkerScriptUrl, type NvbesTrustedScriptUrl } from '@nvbes/web-runtime';
import avatarConversionWorkerUrl from './account.avatar-upload.worker.ts?worker&url';

const MAX_AVATAR_SIZE_BYTES = 5 * 1024 * 1024;
const BROWSER_IMAGE_TYPES = new Set(['image/jpeg', 'image/png', 'image/webp']);
const HEIC_IMAGE_TYPES = new Set([
  'image/heic',
  'image/heif',
  'image/heic-sequence',
  'image/heif-sequence',
]);

function hasHeicExtension(fileName: string): boolean {
  return /\.(heic|heif)$/i.test(fileName);
}

function jpegFileName(fileName: string): string {
  const baseName = fileName.replace(/\.(heic|heif)$/i, '');
  return `${baseName || 'avatar'}.jpg`;
}

interface AvatarConversionResponse {
  blob?: Blob;
  error?: string;
  success: boolean;
}

function createAvatarConversionWorker(): Worker {
  const TrustedWorker = Worker as unknown as new (
    scriptURL: string | URL | NvbesTrustedScriptUrl,
    options?: WorkerOptions,
  ) => Worker;
  return new TrustedWorker(createTrustedWorkerScriptUrl(avatarConversionWorkerUrl), {
    name: 'avatar-heic-converter',
    type: 'module',
  });
}

export function convertHeicInWorker(
  file: File,
  createWorker: () => Worker = createAvatarConversionWorker,
): Promise<Blob> {
  const worker = createWorker();

  return new Promise((resolve, reject) => {
    const fail = (error: Error) => {
      worker.terminate();
      reject(error);
    };

    worker.onmessage = ({ data }: MessageEvent<AvatarConversionResponse>) => {
      worker.terminate();
      if (data.success && data.blob) {
        resolve(data.blob);
        return;
      }
      reject(new Error(data.error ?? 'avatar_conversion_failed'));
    };
    worker.onerror = () => fail(new Error('avatar_conversion_worker_failed'));
    worker.onmessageerror = () => fail(new Error('avatar_conversion_message_failed'));
    worker.postMessage({ file });
  });
}

export async function prepareProfileAvatar(file: File): Promise<File> {
  if (file.size < 1 || file.size > MAX_AVATAR_SIZE_BYTES) {
    throw new Error('invalid_avatar_size');
  }

  const isHeicCandidate =
    HEIC_IMAGE_TYPES.has(file.type.toLowerCase()) || hasHeicExtension(file.name);
  if (!isHeicCandidate) {
    if (!BROWSER_IMAGE_TYPES.has(file.type.toLowerCase())) {
      throw new Error('invalid_avatar_type');
    }
    return file;
  }

  const converted = await convertHeicInWorker(file);
  if (
    !(converted instanceof Blob) ||
    converted.size < 1 ||
    converted.size > MAX_AVATAR_SIZE_BYTES
  ) {
    throw new Error('invalid_converted_avatar_size');
  }

  return new File([converted], jpegFileName(file.name), {
    type: 'image/jpeg',
    lastModified: file.lastModified,
  });
}
