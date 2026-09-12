import { beforeAll, beforeEach, describe, expect, it } from 'vite-plus/test';
import {
  createDpopProof,
  extractNonceFromResponse,
  generateDpopKeyPair,
  getCachedJkt,
  getDpopNonce,
  setDpopNonce,
} from '../dpop';

beforeAll(async () => {
  if (!globalThis.crypto) {
    const { webcrypto } = await import('node:crypto');
    Object.defineProperty(globalThis, 'crypto', {
      value: webcrypto,
      writable: true,
      configurable: true,
    });
  }
});

describe('DPoP SDK Nonce & Proof Handling', () => {
  beforeEach(() => {
    setDpopNonce('');
  });

  it('setDpopNonce and getDpopNonce work properly', () => {
    setDpopNonce('test-nonce-123');
    expect(getDpopNonce()).toBe('test-nonce-123');
  });

  it('extractNonceFromResponse extracts DPoP-Nonce header', () => {
    const headers = new Headers();
    headers.set('dpop-nonce', 'server-nonce-456');
    const nonce = extractNonceFromResponse(headers);

    expect(nonce).toBe('server-nonce-456');
    expect(getDpopNonce()).toBe('server-nonce-456');
  });

  it('generateDpopKeyPair produces a valid keypair with thumbprint', async () => {
    const keyPair = await generateDpopKeyPair();
    expect(keyPair.jkt).toBeTruthy();
    expect(keyPair.publicJwk.kty).toBe('EC');
    expect(getCachedJkt()).toBe(keyPair.jkt);
  });

  it('createDpopProof creates a 3-part JWT proof containing htu, htm and nonce', async () => {
    const keyPair = await generateDpopKeyPair();
    setDpopNonce('active-nonce-789');

    const proof = await createDpopProof(
      keyPair,
      'POST',
      'https://identity.example.com/oauth/token',
      'access-token-abc',
    );

    const parts = proof.split('.');
    expect(parts.length).toBe(3);

    const headerJson = JSON.parse(Buffer.from(parts[0], 'base64url').toString('utf-8'));
    const payloadJson = JSON.parse(Buffer.from(parts[1], 'base64url').toString('utf-8'));

    expect(headerJson.typ).toBe('dpop+jwt');
    expect(headerJson.alg).toBe('ES256');
    expect(headerJson.jwk.kty).toBe('EC');

    expect(payloadJson.htm).toBe('POST');
    expect(payloadJson.htu).toBe('https://identity.example.com/oauth/token');
    expect(payloadJson.nonce).toBe('active-nonce-789');
    expect(payloadJson.ath).toBeTruthy();
  });
});
