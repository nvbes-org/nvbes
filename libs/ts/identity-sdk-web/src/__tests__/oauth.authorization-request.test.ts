import { describe, expect, it, vi } from 'vite-plus/test';
import { createAuthorizationRequest } from '../oauth.authorization-request';
import { MemoryStorage } from '../storage';

describe('pushed authorization requests', () => {
  it('uses PAR, PKCE S256, an explicit audience, and no credentials', async () => {
    const storage = new MemoryStorage();
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(
        JSON.stringify({
          request_uri: 'urn:ietf:params:oauth:request_uri:request-1',
          expires_in: 90,
        }),
        { status: 201, headers: { 'Content-Type': 'application/json' } },
      ),
    );

    const result = await createAuthorizationRequest(
      {
        baseUrl: 'https://identity.example/',
        clientId: 'account-web',
        redirectUri: 'https://account.example/oauth/callback',
        audience: 'nvbes-account-service',
        storage,
        fetchImpl,
        now: () => 1_000,
      },
      {
        scope: 'account:profile:read account:profile:write',
        state: 'state-1',
        returnTo: '/privacy',
      },
    );

    expect(result.authorizationUrl).toBe(
      'https://identity.example/oauth/authorize?client_id=account-web&request_uri=urn%3Aietf%3Aparams%3Aoauth%3Arequest_uri%3Arequest-1',
    );
    expect(storage.getTransaction()).toMatchObject({
      state: 'state-1',
      nonce: null,
      createdAt: 1_000,
      returnTo: '/privacy',
    });

    const [url, init] = fetchImpl.mock.calls[0] ?? [];
    const body = init?.body;
    expect(url).toBe('https://identity.example/oauth/par');
    expect(init?.credentials).toBe('omit');
    expect(body).toBeInstanceOf(URLSearchParams);
    if (!(body instanceof URLSearchParams)) {
      throw new Error('Expected a form-encoded PAR body.');
    }
    expect(body.get('audience')).toBe('nvbes-account-service');
    expect(body.get('code_challenge_method')).toBe('S256');
    expect(body.get('code_challenge')).toBeTruthy();
    expect(body.get('nonce')).toBeNull();
  });

  it('adds an OIDC nonce when openid is requested', async () => {
    const storage = new MemoryStorage();
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(
        JSON.stringify({
          request_uri: 'urn:ietf:params:oauth:request_uri:request-2',
          expires_in: 90,
        }),
        { status: 201, headers: { 'Content-Type': 'application/json' } },
      ),
    );

    await createAuthorizationRequest(
      {
        baseUrl: 'https://identity.example',
        clientId: 'cloud-web',
        redirectUri: 'https://cloud.example/oauth/callback',
        storage,
        fetchImpl,
      },
      { scope: 'openid profile', state: 'state-2', nonce: 'nonce-2' },
    );

    const body = fetchImpl.mock.calls[0]?.[1]?.body;
    if (!(body instanceof URLSearchParams)) {
      throw new Error('Expected a form-encoded PAR body.');
    }
    expect(body.get('nonce')).toBe('nonce-2');
    expect(storage.getTransaction()?.nonce).toBe('nonce-2');
  });

  it('rejects cross-origin return targets before contacting Identity', async () => {
    const fetchImpl = vi.fn<typeof fetch>();

    await expect(
      createAuthorizationRequest(
        {
          baseUrl: 'https://identity.example',
          clientId: 'account-web',
          redirectUri: 'https://account.example/oauth/callback',
          storage: new MemoryStorage(),
          fetchImpl,
        },
        { returnTo: 'https://attacker.example/' },
      ),
    ).rejects.toThrow('same-origin path');

    expect(fetchImpl).not.toHaveBeenCalled();
  });
});
