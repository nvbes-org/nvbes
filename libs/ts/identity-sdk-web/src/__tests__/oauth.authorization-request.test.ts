import { describe, expect, it, vi } from 'vite-plus/test';
import { createAuthorizationRequest } from '../oauth.authorization-request';
import { MemoryStorage } from '../storage';

describe('pushed authorization requests', () => {
  it('sends explicit freshness requirements through PAR, including max_age zero', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(
        JSON.stringify({
          request_uri: 'urn:ietf:params:oauth:request_uri:fresh',
          expires_in: 90,
        }),
        { status: 201 },
      ),
    );
    const config = {
      storage: new MemoryStorage(),
      fetchImpl,
      dpop: false,
      baseUrl: 'https://identity.example',
      clientId: 'account',
      redirectUri: 'https://account.example/callback',
      resource: 'https://api.example',
    };
    for (const maxAge of [-1, 0.5, Number.NaN, Infinity, 86401])
      await expect(createAuthorizationRequest(config, { maxAge })).rejects.toThrow();
    expect(fetchImpl).not.toHaveBeenCalled();
    expect(config.storage.getTransaction()).toBeNull();
    await createAuthorizationRequest(config, { prompt: 'login', maxAge: 0 });
    const body = fetchImpl.mock.calls[0]?.[1]?.body;
    expect(body).toBeInstanceOf(URLSearchParams);
    expect((body as URLSearchParams).get('prompt')).toBe('login');
    expect((body as URLSearchParams).get('max_age')).toBe('0');
  });
  it('rejects invalid protocol inputs before saving state or making a request', async () => {
    const storage = new MemoryStorage();
    const fetchImpl = vi.fn<typeof fetch>();
    const config = {
      baseUrl: 'https://identity.example',
      dpop: false,
      clientId: 'account-web',
      redirectUri: 'https://account.example/oauth/callback',
      resource: 'https://api.example/account',
      storage,
      fetchImpl,
    };
    for (const input of [
      { scope: 'account:read' },
      { state: 'short' },
      { nonce: 'short' },
      { resource: 'https://api.example/account#fragment' },
      { returnTo: '/\\attacker.example' },
      { returnTo: '/\n/attacker.example' },
    ]) {
      await expect(createAuthorizationRequest(config, input)).rejects.toThrow();
      expect(storage.getTransaction()).toBeNull();
    }
    for (const baseUrl of [
      'http://identity.example',
      'https://user:pass@identity.example',
      'https://identity.example/path',
    ]) {
      await expect(createAuthorizationRequest({ ...config, baseUrl })).rejects.toThrow();
    }
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('uses PAR, PKCE S256, an explicit resource, and no credentials', async () => {
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
        dpop: false,
        clientId: 'account-web',
        redirectUri: 'https://account.example/oauth/callback',
        resource: 'https://api.example/account',
        storage,
        fetchImpl,
        now: () => 1_000,
      },
      {
        scope: 'openid account:read account:write',
        state: 'transaction-state-1',
        returnTo: '/privacy',
      },
    );

    expect(result.authorizationUrl).toBe(
      'https://identity.example/oauth/authorize?client_id=account-web&request_uri=urn%3Aietf%3Aparams%3Aoauth%3Arequest_uri%3Arequest-1',
    );
    expect(storage.getTransaction()).toMatchObject({
      state: 'transaction-state-1',
      nonce: expect.any(String),
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
    expect(body.get('resource')).toBe('https://api.example/account');
    expect(body.has('audience')).toBe(false);
    expect(init?.redirect).toBe('error');
    expect(body.get('code_challenge_method')).toBe('S256');
    expect(body.get('code_challenge')).toBeTruthy();
    expect(body.get('nonce')).toBe(storage.getTransaction()?.nonce);
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
        dpop: false,
        clientId: 'account-web',
        redirectUri: 'https://account.example/oauth/callback',
        resource: 'https://identity.example/oauth/userinfo',
        storage,
        fetchImpl,
      },
      { scope: 'openid profile', state: 'transaction-state-2', nonce: 'transaction-nonce-2' },
    );

    const body = fetchImpl.mock.calls[0]?.[1]?.body;
    if (!(body instanceof URLSearchParams)) {
      throw new Error('Expected a form-encoded PAR body.');
    }
    expect(body.get('nonce')).toBe('transaction-nonce-2');
    expect(storage.getTransaction()?.nonce).toBe('transaction-nonce-2');
  });

  it('rejects cross-origin return targets before contacting Identity', async () => {
    const fetchImpl = vi.fn<typeof fetch>();

    await expect(
      createAuthorizationRequest(
        {
          baseUrl: 'https://identity.example',
          dpop: false,
          clientId: 'account-web',
          resource: 'https://api.example/account',
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
