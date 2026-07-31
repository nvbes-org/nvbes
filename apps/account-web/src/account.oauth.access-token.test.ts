import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  clearAccountAccessToken,
  getAccountAccessToken,
  hasAccountScope,
  setAccountAccessToken,
  subscribeAccountAccessToken,
} from './account.oauth.access-token';

describe('Account OAuth access token memory', () => {
  beforeEach(() => {
    clearAccountAccessToken();
  });

  afterEach(() => {
    clearAccountAccessToken();
  });

  it('keeps a valid token only in module memory', () => {
    setAccountAccessToken(
      {
        accessToken: 'access-token',
        tokenType: 'Bearer',
        expiresIn: 300,
        refreshToken: null,
        idToken: null,
        scope: 'account:profile:read account:privacy:read',
        returnTo: '/profile',
      },
      1_000,
    );

    expect(getAccountAccessToken(2_000)).toBe('access-token');
    expect(hasAccountScope('account:privacy:read', 2_000)).toBe(true);
    expect(hasAccountScope('account:profile:write', 2_000)).toBe(false);
  });

  it('clears a token before its expiry boundary', () => {
    const listener = vi.fn();
    const unsubscribe = subscribeAccountAccessToken(listener);
    setAccountAccessToken(
      {
        accessToken: 'access-token',
        tokenType: 'Bearer',
        expiresIn: 10,
        refreshToken: null,
        idToken: null,
        scope: 'account:profile:read',
        returnTo: '/',
      },
      1_000,
    );

    expect(getAccountAccessToken(6_000)).toBeNull();
    expect(listener).toHaveBeenCalledTimes(2);
    unsubscribe();
  });

  it('rejects token types other than bearer', () => {
    expect(() =>
      setAccountAccessToken({
        accessToken: 'access-token',
        tokenType: 'DPoP',
        expiresIn: 300,
        refreshToken: null,
        idToken: null,
        scope: 'account:profile:read',
        returnTo: '/profile',
      }),
    ).toThrow('type de jeton OAuth');
  });
});
