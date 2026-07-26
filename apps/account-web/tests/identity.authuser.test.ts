import { describe, expect, it } from 'vite-plus/test';

import {
  accountHrefForAuthuser,
  accountPathForAuthuser,
  readAuthuser,
} from '../src/identity.authuser';

describe('account index URL helpers', () => {
  it('reads the account index before legacy URL forms', () => {
    expect(readAuthuser('?authuser=4', '/account/2/security')).toBe('2');
    expect(readAuthuser('', '/u/3/account/security')).toBe('3');
  });

  it('keeps the current page when switching accounts', () => {
    expect(accountPathForAuthuser('2', '/account/0')).toBe('/account/2');
    expect(accountPathForAuthuser('2', '/account/0/security')).toBe('/account/2/security');
    expect(accountHrefForAuthuser('2', '/account/0/security', '?tab=sessions&authuser=1')).toBe(
      '/account/2/security?tab=sessions',
    );
  });
});
