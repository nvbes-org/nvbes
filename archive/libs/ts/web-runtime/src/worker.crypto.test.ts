// @vitest-environment node
import { afterEach, expect, it, vi } from 'vite-plus/test';

afterEach(() => vi.unstubAllGlobals());

it('encrypts with interoperable HKDF/AES-GCM and signs using isolated key handles', async () => {
  vi.resetModules();
  const runtime = {
    onmessage: null as ((event: MessageEvent) => Promise<void>) | null,
    postMessage: vi.fn(),
  };
  vi.stubGlobal('self', runtime);
  await import('./worker.crypto');
  const plaintext = new TextEncoder().encode('synthetic payload');
  const secret = new Uint8Array(32).fill(7);
  await runtime.onmessage?.(
    new MessageEvent('message', {
      data: { id: 1, method: 'encrypt', args: [plaintext.buffer, secret.buffer] },
    }),
  );
  const encrypted: unknown = runtime.postMessage.mock.lastCall?.[0];
  expect(encrypted).toMatchObject({
    id: 1,
    result: { encrypted: expect.any(ArrayBuffer), iv: expect.any(Array), salt: expect.any(Array) },
  });
  const result = (encrypted as { result: { encrypted: ArrayBuffer; iv: number[]; salt: number[] } })
    .result;
  expect(result.iv).toHaveLength(12);
  expect(result.salt).toHaveLength(16);
  const material = await crypto.subtle.importKey('raw', secret, 'HKDF', false, ['deriveKey']);
  const key = await crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(result.salt),
      info: new TextEncoder().encode('nvbes-e2ee-request'),
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    ['decrypt'],
  );
  const decoded = await crypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: new Uint8Array(result.iv),
      additionalData: new TextEncoder().encode('nvbes-e2ee-v1'),
    },
    key,
    result.encrypted,
  );
  expect(new TextDecoder().decode(decoded)).toBe('synthetic payload');
  for (const id of [1, 2]) {
    await runtime.onmessage?.(
      new MessageEvent('message', { data: { id: 10, method: 'generateKeyPair', args: [] } }),
    );
    const generated = runtime.postMessage.mock.lastCall?.[0] as {
      result: { keyHandleId: number; publicJwk: JsonWebKey };
    };
    expect(generated.result.keyHandleId).toBe(id);
    expect(generated.result.publicJwk).toMatchObject({ kty: 'EC', crv: 'P-256' });
    expect(generated.result.publicJwk.d).toBeUndefined();
    await runtime.onmessage?.(
      new MessageEvent('message', {
        data: { id: 11, method: 'sign', args: [id, plaintext.buffer] },
      }),
    );
    const signature = runtime.postMessage.mock.lastCall?.[0] as { result: ArrayBuffer };
    const publicKey = await crypto.subtle.importKey(
      'jwk',
      generated.result.publicJwk,
      { name: 'ECDSA', namedCurve: 'P-256' },
      false,
      ['verify'],
    );
    await expect(
      crypto.subtle.verify(
        { name: 'ECDSA', hash: 'SHA-256' },
        publicKey,
        signature.result,
        plaintext,
      ),
    ).resolves.toBe(true);
  }
  await runtime.onmessage?.(
    new MessageEvent('message', {
      data: { id: 12, method: 'sign', args: [999, plaintext.buffer] },
    }),
  );
  expect(runtime.postMessage).toHaveBeenLastCalledWith({
    id: 12,
    error: 'Key handle not found: 999',
  });
});
