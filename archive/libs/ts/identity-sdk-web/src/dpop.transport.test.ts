/// <reference types="node" />
import { webcrypto } from 'node:crypto';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';

beforeEach(() => {
  vi.resetModules();
  vi.stubGlobal('crypto', webcrypto);
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('exports a verifiable public key and signs a token-bound proof with a fresh identifier', async () => {
  const dpop = await import('./dpop');
  expect(dpop.getCachedKeyPair()).toBeNull();
  expect(dpop.getCachedJkt()).toBeNull();
  expect(dpop.getCachedPublicJwk()).toBeNull();
  const key = await dpop.ensureDpopKeyPair();
  expect(await dpop.ensureDpopKeyPair()).toBe(key);
  expect(dpop.getCachedPublicJwk()).toBe(key.publicJwk);
  expect(dpop.getCachedJkt()).toBe(key.jkt);
  const verificationKey = await crypto.subtle.importKey(
    'jwk',
    key.publicJwk,
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['verify'],
  );
  dpop.setDpopNonce('nonce-1');
  const proof = await dpop.createDpopProof(
    key,
    'post',
    'https://identity.test/resource',
    'access-token',
  );
  const [header, payload, signature] = proof.split('.');
  const decodedHeader = JSON.parse(Buffer.from(header, 'base64url').toString());
  const decodedPayload = JSON.parse(Buffer.from(payload, 'base64url').toString());
  expect(decodedHeader).toEqual({
    typ: 'dpop+jwt',
    alg: 'ES256',
    jwk: { kty: 'EC', crv: 'P-256', x: key.publicJwk.x, y: key.publicJwk.y },
  });
  expect(decodedPayload).toMatchObject({
    htm: 'POST',
    htu: 'https://identity.test/resource',
    nonce: 'nonce-1',
    iat: expect.any(Number),
    jti: expect.any(String),
  });
  expect(decodedPayload.ath).toBe(
    Buffer.from(
      await crypto.subtle.digest('SHA-256', new TextEncoder().encode('access-token')),
    ).toString('base64url'),
  );
  expect(
    await crypto.subtle.verify(
      { name: 'ECDSA', hash: 'SHA-256' },
      verificationKey,
      Buffer.from(signature, 'base64url'),
      new TextEncoder().encode(`${header}.${payload}`),
    ),
  ).toBe(true);
  const second = await dpop.createDpopProof(key, 'GET', 'https://identity.test/resource');
  const secondPayload = JSON.parse(Buffer.from(second.split('.')[1], 'base64url').toString());
  expect(secondPayload.jti).not.toBe(decodedPayload.jti);
  expect(secondPayload).not.toHaveProperty('ath');
});

it('attaches proof and authorization, preserves request data and adopts the server nonce', async () => {
  const dpop = await import('./dpop');
  const response = new Response(null, { status: 401, headers: { 'DPoP-Nonce': 'next-nonce' } });
  vi.mocked(fetch).mockResolvedValue(response);
  expect(
    await dpop.dpopFetch('https://identity.test/resource', {
      method: 'POST',
      body: 'body',
      credentials: 'include',
      headers: { 'X-Custom': 'keep' },
      accessToken: 'token',
    }),
  ).toBe(response);
  expect(fetch).toHaveBeenCalledTimes(1);
  const options = vi.mocked(fetch).mock.calls[0][1];
  expect(options).toMatchObject({ method: 'POST', body: 'body', credentials: 'include' });
  const headers = new Headers(options?.headers);
  expect(headers.get('X-Custom')).toBe('keep');
  expect(headers.get('Authorization')).toBe('DPoP token');
  expect(headers.get('DPoP')?.split('.')).toHaveLength(3);
  expect(dpop.getDpopNonce()).toBe('next-nonce');
});

it('permits unsigned transport only when explicitly requested', async () => {
  const dpop = await import('./dpop');
  vi.stubGlobal('crypto', undefined);
  await dpop.dpopFetch('https://identity.test/public', { dpop: false });
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(new Headers(vi.mocked(fetch).mock.calls[0][1]?.headers).has('DPoP')).toBe(false);
});

it('does not send an unsigned request when key generation fails', async () => {
  const dpop = await import('./dpop');
  vi.stubGlobal('crypto', undefined);
  await expect(dpop.dpopFetch('https://identity.test/resource')).rejects.toThrow();
  expect(fetch).not.toHaveBeenCalled();
});

it('never retries an ambiguous network failure, including non-idempotent writes', async () => {
  const dpop = await import('./dpop');
  const failure = new Error('connection lost after acceptance');
  vi.mocked(fetch).mockRejectedValue(failure);
  await expect(
    dpop.dpopFetch('https://identity.test/resource', { method: 'POST', body: 'write' }),
  ).rejects.toBe(failure);
  expect(fetch).toHaveBeenCalledTimes(1);
});

it('uses the configured worker handle and propagates signing failures without network traffic', async () => {
  const dpop = await import('./dpop');
  const worker = {
    generateKeyPair: vi
      .fn()
      .mockResolvedValue({ handle: 17, publicJwk: { kty: 'EC', crv: 'P-256', x: 'x', y: 'y' } }),
    sign: vi.fn().mockResolvedValue(new Uint8Array([251, 255]).buffer),
  };
  dpop.configureDpopCryptoWorker(worker);
  const key = await dpop.ensureDpopKeyPair();
  const proof = await dpop.createDpopProof(key, 'GET', 'https://identity.test/resource');
  expect(proof.endsWith('.-_8')).toBe(true);
  expect(worker.sign).toHaveBeenCalledWith(17, proof.slice(0, proof.lastIndexOf('.')));
  worker.sign.mockRejectedValue(new Error('worker unavailable'));
  await expect(dpop.dpopFetch('https://identity.test/resource')).rejects.toThrow(
    'worker unavailable',
  );
  expect(fetch).not.toHaveBeenCalled();
});

it.each([undefined, {}, { subtle: {} }, { subtle: { generateKey: 1 } }])(
  'rejects incomplete crypto support %j',
  async (value) => {
    const dpop = await import('./dpop');
    vi.stubGlobal('crypto', value);
    expect(dpop.isDpopSupported()).toBe(false);
  },
);
it('recognizes native crypto support', async () => {
  expect((await import('./dpop')).isDpopSupported()).toBe(true);
});
