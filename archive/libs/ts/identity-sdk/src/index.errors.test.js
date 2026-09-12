import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

vi.mock('@nvbes/http-client', () => ({
  createRequestHeaders: (method, headers) => ({ ...headers, 'X-Test-Method': method }),
}));

import { NvbesIdentity } from './index';

function identity() {
  return new NvbesIdentity({
    clientId: 'web-client',
    redirectUri: 'https://app.example.test/callback',
    authorizationUrl: 'https://identity.example.test/oauth/authorize',
    tokenUrl: 'https://identity.example.test/oauth/token',
    userInfoUrl: 'https://identity.example.test/oauth/userinfo',
  });
}

function response(ok, statusText = '', body = {}) {
  return { ok, statusText, json: async () => body };
}

async function authenticatedIdentity(fetchMock) {
  fetchMock.mockResolvedValueOnce(
    response(true, '', {
      access_token: 'access-token',
      expires_in: 900,
      scope: 'openid',
      token_type: 'Bearer',
    }),
  );
  const client = identity();
  await client.exchangeCode('code', 'verifier');
  return client;
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('NvbesIdentity failure contracts', () => {
  it('uses the default OIDC scope and omits a missing confidential secret', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response(true));
    vi.stubGlobal('fetch', fetchMock);
    const client = identity();

    const authorizationUrl = new URL(
      client.getAuthorizationUrl(undefined, 'state', 'challenge', 'nonce'),
    );
    await client.exchangeCode('code', 'verifier');

    expect(authorizationUrl.searchParams.get('scope')).toBe('openid profile email');
    const body = fetchMock.mock.calls[0][1].body;
    expect(body.has('client_secret')).toBe(false);
  });

  it('omits a missing confidential secret from refresh requests', async () => {
    const fetchMock = vi.fn().mockResolvedValue(response(true));
    vi.stubGlobal('fetch', fetchMock);

    await identity().refreshToken('refresh-token');

    const body = fetchMock.mock.calls[0][1].body;
    expect(body.has('client_secret')).toBe(false);
  });

  it.each(['getUserInfo', 'listConsents', 'grantConsent', 'revokeConsent'])(
    'rejects unauthenticated %s calls without a network request',
    async (method) => {
      const fetchMock = vi.fn();
      vi.stubGlobal('fetch', fetchMock);
      const client = identity();

      const operation =
        method === 'grantConsent' || method === 'revokeConsent'
          ? client[method]('analytics', '2026-09')
          : client[method]();

      await expect(operation).rejects.toThrow('No access token available');
      expect(fetchMock).not.toHaveBeenCalled();
    },
  );

  it.each([
    ['getUserInfo', 'Failed to fetch user info'],
    ['listConsents', 'Failed to list consents'],
    ['grantConsent', 'Failed to grant consent'],
    ['revokeConsent', 'Failed to revoke consent'],
  ])('surfaces an authenticated %s endpoint failure', async (method, message) => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    const client = await authenticatedIdentity(fetchMock);
    fetchMock.mockResolvedValueOnce(response(false, 'Service Unavailable'));

    const operation =
      method === 'grantConsent' || method === 'revokeConsent'
        ? client[method]('analytics', '2026-09')
        : client[method]();

    await expect(operation).rejects.toThrow(`${message}: Service Unavailable`);
  });

  it('lists consents without adding an empty query string', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    const client = await authenticatedIdentity(fetchMock);
    fetchMock.mockResolvedValueOnce(
      response(true, '', { consents: [], has_more: false, next_cursor: null }),
    );

    await client.listConsents();

    expect(fetchMock.mock.calls[1][0]).toBe('https://identity.example.test/legal/consents');
  });

  it('does not strip userinfo from the middle of an API base URL', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    fetchMock.mockResolvedValueOnce(
      response(true, '', {
        access_token: 'access-token',
        expires_in: 900,
        scope: 'openid',
        token_type: 'Bearer',
      }),
    );
    const client = new NvbesIdentity({
      clientId: 'web-client',
      redirectUri: 'https://app.example.test/callback',
      authorizationUrl: 'https://identity.example.test/oauth/authorize',
      tokenUrl: 'https://identity.example.test/oauth/token',
      userInfoUrl: 'https://identity.example.test/oauth/userinfo/versioned',
    });
    await client.exchangeCode('code', 'verifier');
    fetchMock.mockResolvedValueOnce(
      response(true, '', { consents: [], has_more: false, next_cursor: null }),
    );

    await client.listConsents();

    expect(fetchMock.mock.calls[1][0]).toBe(
      'https://identity.example.test/oauth/userinfo/versioned/legal/consents',
    );
  });
});
