import { beforeAll, describe, expect, it } from 'vitest';
import { generateCodeChallenge, generateCodeVerifier } from '../pkce';

// Mock crypto for Node.js test environment
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

describe('PKCE', () => {
  it('generateCodeVerifier should produce a base64url string of 43 chars', () => {
    const verifier = generateCodeVerifier();
    expect(verifier).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(verifier.length).toBe(43);
  });

  it('generateCodeVerifier should produce unique values', () => {
    const v1 = generateCodeVerifier();
    const v2 = generateCodeVerifier();
    expect(v1).not.toBe(v2);
  });

  it('generateCodeChallenge should produce a base64url string from verifier', async () => {
    const verifier = generateCodeVerifier();
    const challenge = await generateCodeChallenge(verifier);
    expect(challenge).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(challenge.length).toBeGreaterThanOrEqual(32);
  });

  it('generateCodeChallenge should be deterministic for same verifier', async () => {
    const verifier = generateCodeVerifier();
    const c1 = await generateCodeChallenge(verifier);
    const c2 = await generateCodeChallenge(verifier);
    expect(c1).toBe(c2);
  });
});
