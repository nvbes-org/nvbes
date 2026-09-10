import { afterEach, expect, it, vi } from 'vite-plus/test';
import { encryptRequestBody } from './http.request-e2ee';

afterEach(() => vi.unstubAllGlobals());
const input = {
  body: '{"secret":"synthetic"}',
  secret: '0123456789abcdef0123456789abcdef',
  keyId: 'test-key',
  method: 'post',
  url: 'https://a.test/private?public=1',
};

it('produces a decryptable authenticated envelope bound to method and path', async () => {
  const encrypted = await encryptRequestBody(input);
  const decode = (name: string): Uint8Array<ArrayBuffer> => {
    const value = encrypted.headers.get(name);
    expect(value).not.toBeNull();
    return Uint8Array.from(atob(value ?? ''), (character) => character.charCodeAt(0));
  };
  const salt = decode('X-Nvbes-E2ee-Salt');
  const nonce = decode('X-Nvbes-E2ee-Nonce');
  expect(salt).toHaveLength(16);
  expect(nonce).toHaveLength(12);
  const material = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(input.secret),
    'HKDF',
    false,
    ['deriveKey'],
  );
  const key = await crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt,
      info: new TextEncoder().encode('nvbes/request-body-e2ee/v1'),
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    ['decrypt'],
  );
  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: nonce, additionalData: new TextEncoder().encode('POST /private') },
    key,
    encrypted.body,
  );
  expect(new TextDecoder().decode(plaintext)).toBe(input.body);
  await expect(
    crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: nonce, additionalData: new TextEncoder().encode('GET /private') },
      key,
      encrypted.body,
    ),
  ).rejects.toThrow();
});

it.each([undefined, {}])('rejects unavailable Web Crypto', async (cryptoValue) => {
  vi.stubGlobal('crypto', cryptoValue);
  await expect(encryptRequestBody(input)).rejects.toThrow('Request E2EE requires Web Crypto');
});

it('rejects a secret just below the minimum length', async () => {
  await expect(encryptRequestBody({ ...input, secret: 'a'.repeat(31) })).rejects.toThrow(
    'at least 32 characters',
  );
});
