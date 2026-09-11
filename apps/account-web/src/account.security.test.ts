import { describe, expect, it } from 'vite-plus/test';
import { parseAccountConfig } from './account.config';
import { parseAccountCallback } from './account.callback';
import { parseAccountProfile } from './account.profile';

describe('Account trust boundaries', () => {
  const config = {
    identityOrigin: 'https://identity.example',
    accountApiOrigin: 'https://account.example',
    clientId: 'account-web',
  };
  it('accepts explicit HTTPS origins and loopback development', () => {
    expect(parseAccountConfig(config)).toEqual(config);
    expect(
      parseAccountConfig({ ...config, identityOrigin: 'http://127.0.0.1:4200' }).identityOrigin,
    ).toBe('http://127.0.0.1:4200');
  });
  it.each([
    'http://identity.example',
    'https://user:pass@identity.example',
    'https://identity.example/evil',
    'https://identity.example/?redirect=x',
    'https://identity.example/#x',
    '//identity.example',
    ' https://identity.example',
  ])('rejects unsafe configured endpoint %s', (identityOrigin) => {
    expect(() => parseAccountConfig({ ...config, identityOrigin })).toThrow();
    expect(() => parseAccountConfig({ ...config, accountApiOrigin: identityOrigin })).toThrow();
  });
  it.each([
    'code=x',
    'code=x&state=wrong',
    'code=x&state=s&state=s',
    'code=x&code=y&state=s',
    'code=x&error=access_denied&state=s',
    'error=server_error&state=s',
    'code=x&state=s#token',
  ])('rejects malformed or unbound callback %s', (query) => {
    expect(() =>
      parseAccountCallback(new URL(`https://account.example/oauth/callback?${query}`), 's'),
    ).toThrow();
  });
  it('accepts a bound code or an explicit denial', () => {
    expect(parseAccountCallback(new URL('https://account.example/?code=x&state=s'), 's')).toEqual({
      kind: 'code',
      code: 'x',
      state: 's',
    });
    expect(
      parseAccountCallback(new URL('https://account.example/?error=access_denied&state=s'), 's'),
    ).toEqual({ kind: 'denied' });
  });
  it('requires a profile matching the verified Identity subject', () => {
    const user = {
      id: 'subject',
      display_name: 'Alice',
      firstname: null,
      lastname: null,
      username: null,
      region: null,
    };
    expect(parseAccountProfile({ user }, 'subject').displayName).toBe('Alice');
    expect(() => parseAccountProfile({ user }, 'other')).toThrow();
    expect(() => parseAccountProfile({ user: { ...user, firstname: {} } }, 'subject')).toThrow();
  });
});
