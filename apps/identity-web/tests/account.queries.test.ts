import { describe, expect, it } from 'vite-plus/test';

import { accountQueryKeys } from '../src/account.queries';
import { authuserSearch, readAuthuser } from '../src/identity.authuser';

describe('account query keys', () => {
  it('scopes account data by active authuser', () => {
    expect(accountQueryKeys.context('0')).toEqual(['identity', 'account', '0', 'context']);
    expect(accountQueryKeys.context('1')).toEqual(['identity', 'account', '1', 'context']);
    expect(accountQueryKeys.sessions('1')).toEqual(['identity', 'account', '1', 'sessions']);
    expect(accountQueryKeys.personalInfo('2')).toEqual([
      'identity',
      'account',
      '2',
      'personal-info',
    ]);
  });
});

describe('authuser search helpers', () => {
  it('reads the selected account from the URL search', () => {
    expect(readAuthuser('?authuser=3&from=%2Faccount')).toBe('3');
    expect(readAuthuser('?from=%2Faccount')).toBe('0');
  });

  it('keeps default account URLs clean', () => {
    expect(authuserSearch('0')).toBeUndefined();
    expect(authuserSearch('2')).toEqual({ authuser: '2' });
  });
});
