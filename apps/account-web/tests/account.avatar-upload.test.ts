import { describe, expect, it, vi } from 'vite-plus/test';

import { convertHeicInWorker, prepareProfileAvatar } from '../src/account.avatar-upload';

class AvatarConversionWorker {
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onmessageerror: ((event: MessageEvent) => void) | null = null;
  postMessage = vi.fn();
  terminate = vi.fn();

  respond(data: unknown) {
    this.onmessage?.(new MessageEvent('message', { data }));
  }
}

describe('prepareProfileAvatar', () => {
  it('keeps browser-compatible images unchanged', async () => {
    const jpeg = new File(['jpeg'], 'portrait.jpg', { type: 'image/jpeg' });

    await expect(prepareProfileAvatar(jpeg)).resolves.toBe(jpeg);
  });

  it('runs HEIC conversion through a dedicated worker', async () => {
    const heic = new File(['heic'], 'portrait.HEIC', { type: '' });
    const worker = new AvatarConversionWorker();
    const conversion = convertHeicInWorker(heic, () => worker as unknown as Worker);
    worker.respond({ blob: new Blob(['jpeg'], { type: 'image/jpeg' }), success: true });

    const converted = await conversion;

    expect(converted.type).toBe('image/jpeg');
    expect(worker.postMessage).toHaveBeenCalledWith({ file: heic });
    expect(worker.terminate).toHaveBeenCalledOnce();
  });

  it('forwards conversion failures and terminates the worker', async () => {
    const invalid = new File(['not-heic'], 'portrait.heic', { type: 'image/heic' });
    const worker = new AvatarConversionWorker();
    const conversion = convertHeicInWorker(invalid, () => worker as unknown as Worker);
    worker.respond({ error: 'invalid_heic_image', success: false });

    await expect(conversion).rejects.toThrow('invalid_heic_image');
    expect(worker.terminate).toHaveBeenCalledOnce();
  });
});
