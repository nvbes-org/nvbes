import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

vi.mock('@nvbes/http-client', () => ({
  createRequestHeaders: (method, headers) => ({ ...headers, 'X-Test-Method': method }),
}));

import { NvbesIdentity } from './index';

function identity(overrides = {}) {
  return new NvbesIdentity({
    clientId: 'cloud-web',
    redirectUri: 'https://drive.example/callback',
    authorizationUrl: 'https://identity.example/oauth/authorize',
    tokenUrl: 'https://identity.example/oauth/token',
    userInfoUrl: 'https://identity.example/oauth/userinfo',
    ...overrides,
  });
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('NvbesIdentity', () => {
  it('builds authorization-code URLs with S256 PKCE', () => {
    const url = new URL(
      identity().getAuthorizationUrl('openid email', 'state-1', 'challenge-1', 'nonce-1'),
    );

    expect(url.searchParams.get('response_type')).toBe('code');
    expect(url.searchParams.get('client_id')).toBe('cloud-web');
    expect(url.searchParams.get('redirect_uri')).toBe('https://drive.example/callback');
    expect(url.searchParams.get('state')).toBe('state-1');
    expect(url.searchParams.get('code_challenge')).toBe('challenge-1');
    expect(url.searchParams.get('code_challenge_method')).toBe('S256');
    expect(url.searchParams.get('nonce')).toBe('nonce-1');
  });

  it.each([
    {
      challenge: 'challenge',
      message: 'OAuth state is required.',
      missing: 'state',
      nonce: 'nonce',
      state: ' ',
    },
    {
      challenge: ' ',
      message: 'PKCE code challenge is required.',
      missing: 'challenge',
      nonce: 'nonce',
      state: 'state',
    },
    {
      challenge: 'challenge',
      message: 'OIDC nonce is required.',
      missing: 'nonce',
      nonce: '\t',
      state: 'state',
    },
  ])(
    'rejects a missing OAuth $missing correlation value',
    ({ state, challenge, nonce, message }) => {
      expect(() => identity().getAuthorizationUrl('openid', state, challenge, nonce)).toThrow(
        message,
      );
    },
  );

  it('exchanges an authorization code with PKCE and maps the token response', async () => {
    const fetchMock = tokenFetch();
    vi.stubGlobal('fetch', fetchMock);

    const token = await identity({ clientSecret: 'confidential-secret' }).exchangeCode(
      'authorization-code',
      'pkce-verifier',
    );

    expect(token).toEqual({
      accessToken: 'access-2',
      expiresIn: 900,
      idToken: 'id-2',
      refreshToken: 'refresh-2',
      scope: 'openid email',
      tokenType: 'Bearer',
    });
    const body = fetchMock.mock.calls[0][1].body;
    expect(fetchMock.mock.calls[0][1].method).toBe('POST');
    expect(fetchMock.mock.calls[0][1].headers).toEqual({
      'Content-Type': 'application/x-www-form-urlencoded',
      'X-Test-Method': 'POST',
    });
    expect(body.get('grant_type')).toBe('authorization_code');
    expect(body.get('code')).toBe('authorization-code');
    expect(body.get('code_verifier')).toBe('pkce-verifier');
    expect(body.get('client_secret')).toBe('confidential-secret');
  });

  it('rejects an empty PKCE verifier before issuing a network request', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    await expect(identity().exchangeCode('code', ' ')).rejects.toThrow(
      'PKCE code verifier is required',
    );
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('refreshes tokens through the refresh_token grant', async () => {
    const fetchMock = tokenFetch();
    vi.stubGlobal('fetch', fetchMock);

    const token = await identity({ clientSecret: 'refresh-secret' }).refreshToken('refresh-1');

    expect(token.accessToken).toBe('access-2');
    expect(fetchMock).toHaveBeenCalledWith(
      'https://identity.example/oauth/token',
      expect.objectContaining({
        method: 'POST',
        body: expect.any(URLSearchParams),
      }),
    );
    const body = fetchMock.mock.calls[0][1].body;
    expect(body.get('grant_type')).toBe('refresh_token');
    expect(body.get('refresh_token')).toBe('refresh-1');
    expect(body.get('client_secret')).toBe('refresh-secret');
    expect(fetchMock.mock.calls[0][1].headers).toEqual({
      'Content-Type': 'application/x-www-form-urlencoded',
      'X-Test-Method': 'POST',
    });
  });

  it.each([
    ['exchangeCode', 'Token exchange failed'],
    ['refreshToken', 'Token refresh failed'],
  ])('surfaces token endpoint failures from %s', async (method, expectedMessage) => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: false, statusText: 'Service Unavailable' }),
    );
    const client = identity();

    const operation =
      method === 'exchangeCode'
        ? client.exchangeCode('code', 'verifier')
        : client.refreshToken('refresh');
    await expect(operation).rejects.toThrow(`${expectedMessage}: Service Unavailable`);
  });

  it('keeps tokens in memory and clears the authenticated state on logout', async () => {
    vi.stubGlobal('fetch', tokenFetch());
    const client = identity();

    expect(client.isAuthenticated()).toBe(false);
    await expect(client.getAccessToken()).rejects.toThrow('No access token');

    await client.exchangeCode('code', 'verifier');
    expect(client.isAuthenticated()).toBe(true);
    await expect(client.getAccessToken()).resolves.toBe('access-2');
    expect(client.loadToken()).toBe(false);
    expect(client.saveToken()).toBeUndefined();

    client.logout();
    expect(client.isAuthenticated()).toBe(false);
  });

  it('maps OIDC userinfo only after a token is available', async () => {
    const fetchMock = tokenFetch();
    fetchMock.mockResolvedValueOnce(tokenResponse()).mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        sub: 'principal-1',
        email: 'person@example.test',
        name: 'Test Person',
        email_verified: true,
        workspace_id: 'workspace-1',
      }),
    });
    vi.stubGlobal('fetch', fetchMock);
    const client = identity();

    await expect(client.getUserInfo()).rejects.toThrow('No access token');
    await client.exchangeCode('code', 'verifier');
    await expect(client.getUserInfo()).resolves.toEqual({
      email: 'person@example.test',
      emailVerified: true,
      id: 'principal-1',
      name: 'Test Person',
      workspaceId: 'workspace-1',
    });
    expect(fetchMock.mock.calls[1][1].headers.Authorization).toBe('Bearer access-2');
  });

  it('uses authenticated, encoded consent endpoints and exact request bodies', async () => {
    const fetchMock = tokenFetch();
    fetchMock
      .mockResolvedValueOnce(tokenResponse())
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ consents: [], has_more: false, next_cursor: null }),
      })
      .mockResolvedValueOnce({ ok: true, json: async () => ({ id: 'consent-1' }) })
      .mockResolvedValueOnce({ ok: true });
    vi.stubGlobal('fetch', fetchMock);
    const client = identity();
    await client.exchangeCode('code', 'verifier');

    await client.listConsents({ cursor: 'next/page', limit: 25 });
    await client.grantConsent('analytics', '2026-07');
    await client.revokeConsent('analytics', '2026-07');

    expect(fetchMock.mock.calls[1][0]).toBe(
      'https://identity.example/legal/consents?limit=25&cursor=next%2Fpage',
    );
    expect(fetchMock.mock.calls[1][1]).toEqual({
      headers: { Authorization: 'Bearer access-2' },
    });
    expect(fetchMock.mock.calls[2][0]).toBe('https://identity.example/legal/consent');
    expect(fetchMock.mock.calls[2][1].method).toBe('POST');
    expect(fetchMock.mock.calls[2][1].headers).toEqual({
      Authorization: 'Bearer access-2',
      'Content-Type': 'application/json',
      'X-Test-Method': 'POST',
    });
    expect(JSON.parse(fetchMock.mock.calls[2][1].body)).toEqual({
      consent_type: 'analytics',
      document_version: '2026-07',
    });
    expect(fetchMock.mock.calls[3][0]).toBe('https://identity.example/legal/consent/revoke');
    expect(fetchMock.mock.calls[3][1]).toEqual({
      body: JSON.stringify({ consent_type: 'analytics', document_version: '2026-07' }),
      headers: {
        Authorization: 'Bearer access-2',
        'Content-Type': 'application/json',
        'X-Test-Method': 'POST',
      },
      method: 'POST',
    });
  });
});

function tokenFetch() {
  return vi.fn().mockResolvedValue(tokenResponse());
}

function tokenResponse() {
  return {
    ok: true,
    json: async () => ({
      access_token: 'access-2',
      expires_in: 900,
      id_token: 'id-2',
      refresh_token: 'refresh-2',
      scope: 'openid email',
      token_type: 'Bearer',
    }),
  };
}
