import { heicTo, isHeic } from 'heic-to/next';

interface AvatarConversionRequest {
  file: File;
}

type AvatarConversionResponse = { blob: Blob; success: true } | { error: string; success: false };

interface AvatarWorkerScope {
  onmessage: ((event: MessageEvent<AvatarConversionRequest>) => void) | null;
  postMessage(message: AvatarConversionResponse): void;
}

const workerScope = self as unknown as AvatarWorkerScope;

workerScope.onmessage = async ({ data }) => {
  try {
    if (!(await isHeic(data.file))) {
      throw new Error('invalid_heic_image');
    }

    const converted = await heicTo({
      blob: data.file,
      type: 'image/jpeg',
      quality: 0.9,
    });
    if (!(converted instanceof Blob)) {
      throw new Error('invalid_converted_avatar');
    }

    workerScope.postMessage({ blob: converted, success: true });
  } catch (error) {
    workerScope.postMessage({
      error: error instanceof Error ? error.message : 'avatar_conversion_failed',
      success: false,
    });
  }
};
