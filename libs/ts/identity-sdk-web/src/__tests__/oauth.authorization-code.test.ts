import { describe, expect, it, vi } from 'vite-plus/test';
import { exchangeAuthorizationCode } from '../oauth.authorization-code';
import { MemoryStorage } from '../storage';
import { idToken, jwksResponse } from './oidc.fixture';

describe('Authorization Code with PKCE', () => {
  it('exchanges the code and clears the one-time transaction', async () => {
    const storage = new MemoryStorage();
    storage.saveTransaction({
      state: 'expected-state',
      codeVerifier: 'verifier',
      nonce: 'expected-nonce',
      createdAt: 1_000,
      returnTo: '/profile',
    });
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockImplementation(jwksResponse)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            access_token: 'account-access-token',
            id_token: await idToken('expected-nonce', 'account-access-token', {
              iat: 2,
              exp: 302,
              auth_time: 1,
            }),
            token_type: 'Bearer',
            expires_in: 300,
            refresh_token: 'refresh-token',
            scope: 'account:profile:read',
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      );

    const result = await exchangeAuthorizationCode(
      {
        baseUrl: 'https://identity.example/',
        clientId: 'account-web',
        redirectUri: 'https://account.example/oauth/callback',
        storage,
        fetchImpl,
        now: () => 2_000,
      },
      { code: 'authorization-code', state: 'expected-state' },
    );

    expect(result).toMatchObject({
      accessToken: 'account-access-token',
      returnTo: '/profile',
    });
    expect(storage.getTransaction()).toBeNull();
    const [url, init] = fetchImpl.mock.calls[0] ?? [];
    expect(url).toBe('https://identity.example/oauth/token');
    expect(init?.credentials).toBe('omit');
    if (!(init?.body instanceof URLSearchParams)) throw new Error('Expected OAuth form body');
    expect(init.body.get('code_verifier')).toBe('verifier');
  });

  it('rejects a callback with a mismatched state before network access', async () => {
    const storage = new MemoryStorage();
    storage.saveTransaction({
      state: 'expected-state',
      codeVerifier: 'verifier',
      nonce: null,
      createdAt: 1_000,
      returnTo: '/',
    });
    const fetchImpl = vi.fn<typeof fetch>();

    await expect(
      exchangeAuthorizationCode(
        {
          baseUrl: 'https://identity.example',
          clientId: 'account-web',
          redirectUri: 'https://account.example/oauth/callback',
          storage,
          fetchImpl,
          now: () => 2_000,
        },
        { code: 'authorization-code', state: 'attacker-state' },
      ),
    ).rejects.toThrow('OAuth state validation failed');

    expect(fetchImpl).not.toHaveBeenCalled();
    expect(storage.getTransaction()?.codeVerifier).toBe('verifier');
  });

  it('rejects and clears an expired transaction', async () => {
    const storage = new MemoryStorage();
    storage.saveTransaction({
      state: 'expected-state',
      codeVerifier: 'verifier',
      nonce: null,
      createdAt: 1_000,
      returnTo: '/',
    });
    const fetchImpl = vi.fn<typeof fetch>();

    await expect(
      exchangeAuthorizationCode(
        {
          baseUrl: 'https://identity.example',
          clientId: 'account-web',
          redirectUri: 'https://account.example/oauth/callback',
          storage,
          fetchImpl,
          now: () => 1_000 + 16 * 60 * 1_000,
        },
        { code: 'authorization-code', state: 'expected-state' },
      ),
    ).rejects.toThrow('OAuth authorization transaction expired');

    expect(fetchImpl).not.toHaveBeenCalled();
    expect(storage.getTransaction()).toBeNull();
  });
});
