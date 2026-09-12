import { describe, expect, it, vi } from 'vite-plus/test';
import { exchangeAuthorizationCode } from '../oauth.authorization-code';
import { MemoryStorage } from '../storage';

describe('Authorization Code with PKCE', () => {
  it('exchanges the code and clears the one-time transaction', async () => {
    const storage = new MemoryStorage();
    storage.saveTransaction({
      state: 'expected-state',
      codeVerifier: 'verifier',
      nonce: null,
      createdAt: 1_000,
      returnTo: '/profile',
    });
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(
        JSON.stringify({
          access_token: 'account-access-token',
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
    expect(String(init?.body)).toContain('code_verifier=verifier');
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
