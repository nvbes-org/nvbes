import type { OAuthClient } from './account.schemas';
import {
  AccountIdentityClient,
  type IdentityClientOptions,
  type RequestOptions,
} from './identity.account-client';

export const accountClient = new AccountIdentityClient();

export function createAccountIdentityClient(
  options?: IdentityClientOptions,
): AccountIdentityClient {
  return new AccountIdentityClient(options);
}

export function listOAuthClients(options?: RequestOptions): Promise<OAuthClient[]> {
  return accountClient.listOAuthClients(options);
}

export function revokeOAuthClient(clientId: string): Promise<void> {
  return accountClient.revokeOAuthClient(clientId);
}
