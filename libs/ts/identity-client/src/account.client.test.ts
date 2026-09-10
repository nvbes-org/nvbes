import { describe, expect, it } from 'vite-plus/test';
import { accountClient, createAccountIdentityClient } from './account.client';
import { AccountIdentityClient } from './identity.account-client';
import * as publicApi from './index';

describe('Account client boundary', () => {
  it('does not instantiate the Enterprise client for Account consumers', () => {
    expect(accountClient).toBeInstanceOf(AccountIdentityClient);
    expect(Object.keys(publicApi).some((key) => /Enterprise|Federat|AccessReview/.test(key))).toBe(
      false,
    );
    expect('IdentityClient' in publicApi).toBe(false);
    expect('createIdentityClient' in publicApi).toBe(false);
    expect(createAccountIdentityClient()).toBeInstanceOf(AccountIdentityClient);
  });
});
