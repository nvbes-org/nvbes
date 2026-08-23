import { describe, expect, it } from 'vite-plus/test';
import { ACCOUNT_WEB_PATHS } from './account.routes';

describe('Account Web route boundary', () => {
  it('owns Account self-service pages and the OAuth callback', () => {
    expect(Object.values(ACCOUNT_WEB_PATHS)).toEqual([
      '/',
      '/oauth/callback',
      '/profile',
      '/preferences',
      '/notifications',
      '/privacy',
      '/security',
      '/security/sessions',
      '/emails',
      '/connected-apps',
      '/forgot-password',
      '/reset-password',
    ]);
  });

  it('keeps hosted identification challenges outside Account', () => {
    const routes = Object.values(ACCOUNT_WEB_PATHS).join(' ');
    expect(routes).not.toMatch(/login|register|oauth-consent|challenge|accountIndex|authuser/u);
  });
});
