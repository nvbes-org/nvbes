import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

vi.mock('@nvbes/http-client', () => ({
  createRequestHeaders: (_method, headers) => headers,
}));

import { NvbesIdentity } from './index';

function identity() {
  return new NvbesIdentity({
    clientId: 'cloud-web',
    redirectUri: 'https://drive.example/callback',
    authorizationUrl: 'https://identity.example/oauth/authorize',
    tokenUrl: 'https://identity.example/oauth/token',
    userInfoUrl: 'https://identity.example/oauth/userinfo',
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('NvbesIdentity', () => {
  it('builds authorization-code URLs with S256 PKCE', () => {
    const url = new URL(identity().getAuthorizationUrl('openid email', 'state-1', 'challenge-1'));

    expect(url.searchParams.get('response_type')).toBe('code');
    expect(url.searchParams.get('client_id')).toBe('cloud-web');
    expect(url.searchParams.get('redirect_uri')).toBe('https://drive.example/callback');
    expect(url.searchParams.get('code_challenge')).toBe('challenge-1');
    expect(url.searchParams.get('code_challenge_method')).toBe('S256');
  });

  it('refreshes tokens through the refresh_token grant', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        access_token: 'access-2',
        token_type: 'Bearer',
        expires_in: 900,
        refresh_token: 'refresh-2',
        scope: 'openid email',
      }),
    });
    vi.stubGlobal('fetch', fetchMock);

    const token = await identity().refreshToken('refresh-1');

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
  });
});
