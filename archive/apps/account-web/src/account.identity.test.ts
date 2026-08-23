import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { accountIdentityClient } from './account.identity';
import { clearAccountAccessToken, setAccountAccessToken } from './account.oauth.access-token';

afterEach(() => {
  clearAccountAccessToken();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

describe('Account to Identity transport', () => {
  it('uses the OAuth bearer token without browser-session context', async () => {
    vi.stubEnv('VITE_IDENTITY_SERVICE_BASE_URL', 'https://identity.example.test');
    setAccountAccessToken({
      accessToken: 'account-oauth-token',
      tokenType: 'Bearer',
      expiresIn: 300,
      refreshToken: null,
      idToken: null,
      scope: 'account:email:read',
      returnTo: '/emails',
    });
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          emails: [],
          primary_min_age_hours: 24,
          next_cursor: null,
          has_more: false,
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } },
      ),
    );

    await accountIdentityClient().listEmails();

    const request = fetchMock.mock.calls[0];
    expect(request?.[0]).toBe('https://identity.example.test/auth/me/emails');
    const init = request?.[1];
    const headers = new Headers(init?.headers);
    expect(headers.get('Authorization')).toBe('Bearer account-oauth-token');
    expect(headers.has('Cookie')).toBe(false);
    expect(headers.has('X-Auth-User')).toBe(false);
    expect(headers.has('X-CSRF-Token')).toBe(false);
    expect(init?.credentials).toBe('omit');
  });
});
