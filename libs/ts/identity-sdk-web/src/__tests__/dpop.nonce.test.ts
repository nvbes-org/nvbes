import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  createDpopProof,
  dpopFetch,
  extractNonceFromResponse,
  generateDpopKeyPair,
  getCachedJkt,
  getDpopNonce,
  setDpopNonce,
} from '../dpop';

const server = 'https://identity.example.com';
afterEach(() => vi.restoreAllMocks());

describe('DPoP SDK Nonce & Proof Handling', () => {
  beforeEach(() => {
    setDpopNonce('', server);
  });

  it('setDpopNonce and getDpopNonce work properly', () => {
    setDpopNonce('test-nonce-123', server);
    expect(getDpopNonce(server)).toBe('test-nonce-123');
    expect(getDpopNonce('https://account.example.com')).toBeNull();
  });

  it('extractNonceFromResponse extracts DPoP-Nonce header', () => {
    const headers = new Headers();
    headers.set('dpop-nonce', 'server-nonce-456');
    const nonce = extractNonceFromResponse(headers, server);

    expect(nonce).toBe('server-nonce-456');
    expect(getDpopNonce(server)).toBe('server-nonce-456');
  });

  it('generateDpopKeyPair produces a valid keypair with thumbprint', async () => {
    const keyPair = await generateDpopKeyPair();
    expect(keyPair.jkt).toBeTruthy();
    expect(keyPair.publicJwk.kty).toBe('EC');
    expect(getCachedJkt()).toBe(keyPair.jkt);
    if (!keyPair.usingWorker) {
      expect(keyPair.keyPair.privateKey.extractable).toBe(false);
      await expect(crypto.subtle.exportKey('jwk', keyPair.keyPair.privateKey)).rejects.toThrow();
    }
  });

  it('createDpopProof creates a 3-part JWT proof containing htu, htm and nonce', async () => {
    const keyPair = await generateDpopKeyPair();
    setDpopNonce('active-nonce-789', server);

    const proof = await createDpopProof(
      keyPair,
      'POST',
      'https://identity.example.com/oauth/token?ignored=true#fragment',
      'access-token-abc',
    );

    const parts = proof.split('.');
    expect(parts.length).toBe(3);

    const headerJson = JSON.parse(atob(parts[0].replace(/-/g, '+').replace(/_/g, '/')));
    const payloadJson = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/')));

    expect(headerJson.typ).toBe('dpop+jwt');
    expect(headerJson.alg).toBe('ES256');
    expect(headerJson.jwk.kty).toBe('EC');

    expect(payloadJson.htm).toBe('POST');
    expect(payloadJson.htu).toBe('https://identity.example.com/oauth/token');
    expect(payloadJson.nonce).toBe('active-nonce-789');
    expect(payloadJson.ath).toBeTruthy();
  });

  it('never sends a request when signing fails, and never retries transport failures', async () => {
    await generateDpopKeyPair();
    const transport = vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('network'));
    const signer = vi.spyOn(crypto.subtle, 'sign').mockRejectedValueOnce(new Error('signing'));
    await expect(
      dpopFetch(`${server}/api`, { method: 'POST', accessToken: 'secret' }),
    ).rejects.toThrow('signing');
    expect(transport).not.toHaveBeenCalled();
    signer.mockRestore();
    await expect(
      dpopFetch(`${server}/api`, { method: 'POST', accessToken: 'secret' }),
    ).rejects.toThrow('network');
    expect(transport).toHaveBeenCalledTimes(1);
    expect(transport.mock.calls[0][1]).toMatchObject({ redirect: 'error', credentials: 'omit' });
  });
});
