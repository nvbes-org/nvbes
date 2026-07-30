import { describe, expect, it } from 'vite-plus/test';
import { accountClient, createAccountIdentityClient } from './account.client';
import { AccountIdentityClient } from './identity.account-client';
import { IdentityClient } from './identity.enterprise-client';

describe('Account client boundary', () => {
  it('does not instantiate the Enterprise client for Account consumers', () => {
    expect(accountClient).toBeInstanceOf(AccountIdentityClient);
    expect(accountClient).not.toBeInstanceOf(IdentityClient);
    expect(createAccountIdentityClient()).toBeInstanceOf(AccountIdentityClient);
  });
});
