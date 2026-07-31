import { describe, expect, it } from 'vite-plus/test';
import { ACCOUNT_WEB_PATHS } from './account.routes';

describe('Account Web route boundary', () => {
  it('exposes only Account resource pages and the OAuth callback', () => {
    expect(Object.values(ACCOUNT_WEB_PATHS)).toEqual([
      '/',
      '/oauth/callback',
      '/profile',
      '/preferences',
      '/notifications',
      '/privacy',
    ]);
  });

  it('does not expose an Identity or multi-account route', () => {
    const routes = Object.values(ACCOUNT_WEB_PATHS).join(' ');
    expect(routes).not.toMatch(
      /login|register|password|session|linked-app|mfa|webauthn|email|accountIndex|authuser/u,
    );
  });
});
