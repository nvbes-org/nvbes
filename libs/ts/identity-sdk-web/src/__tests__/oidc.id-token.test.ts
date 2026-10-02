import { describe, expect, it, vi } from 'vite-plus/test';
import { verifyIdToken } from '../oidc.id-token';
import { idToken, jwksResponse } from './oidc.fixture';

const config = {
  issuer: 'https://identity.example',
  clientId: 'account-web',
  nonce: 'transaction-nonce',
  accessToken: 'access-token',
};

describe('OIDC ID token validation', () => {
  it('accepts signed identity claims bound to the client, nonce and access token', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockImplementation(jwksResponse);
    const identity = await verifyIdToken({
      ...config,
      token: await idToken(config.nonce, config.accessToken),
      fetchImpl,
    });
    expect(identity.subject).toBe('principal');
    expect(fetchImpl.mock.calls[0][0]).toBe('https://identity.example/oauth/jwks');
    expect(fetchImpl.mock.calls[0][1]).toMatchObject({ credentials: 'omit', redirect: 'error' });
  });

  it('rejects valid signatures carrying wrong claims or a substituted access token', async () => {
    const now = Math.floor(Date.now() / 1000);
    for (const claims of [
      { iss: 'https://other.example' },
      { aud: 'other-client' },
      { nonce: 'other-nonce' },
      { exp: now - 1 },
      { iat: now + 120 },
      { auth_time: now + 500 },
      { azp: 'other-client' },
      { sub: '' },
      { at_hash: 'incorrect' },
    ]) {
      await expect(
        verifyIdToken({
          ...config,
          token: await idToken(config.nonce, config.accessToken, claims),
          fetchImpl: jwksResponse,
        }),
      ).rejects.toThrow();
    }
    await expect(
      verifyIdToken({
        ...config,
        token: await idToken(config.nonce, 'different-access-token'),
        fetchImpl: jwksResponse,
      }),
    ).rejects.toThrow('binding');
  });

  it('rejects forged signatures, token type confusion and unknown signing keys', async () => {
    const token = await idToken(config.nonce, config.accessToken);
    const parts = token.split('.');
    parts[2] = (parts[2][0] === 'A' ? 'B' : 'A') + parts[2].slice(1);
    for (const bad of [
      parts.join('.'),
      await idToken(config.nonce, config.accessToken, {}, { typ: 'at+jwt' }),
      await idToken(config.nonce, config.accessToken, {}, { kid: 'unknown' }),
    ]) {
      await expect(
        verifyIdToken({ ...config, token: bad, fetchImpl: jwksResponse }),
      ).rejects.toThrow();
    }
  });

  it('fails on unavailable or oversized keys without a token-derived fallback URL', async () => {
    const token = await idToken(config.nonce, config.accessToken);
    for (const response of [new Response('', { status: 503 }), new Response('x'.repeat(65_537))]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(response);
      await expect(verifyIdToken({ ...config, token, fetchImpl })).rejects.toThrow();
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    }
  });
});
