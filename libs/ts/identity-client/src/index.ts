export * from './account.schemas';
export * from './enterprise.access-reviews.schemas';
export * from './enterprise.client';
export * from './enterprise.schemas';
export * from './enterprise.trust.schemas';
export * from './federation.schemas';
export {
  AccountIdentityClient,
  type IdentityClientOptions,
  type RequestOptions,
} from './identity.account-client';
export { IdentityClient } from './identity.enterprise-client';

import type { OAuthClient } from './account.schemas';
import { IdentityClient } from './identity.enterprise-client';
import type { RequestOptions } from './identity.account-client';

export const identityClient = new IdentityClient();

export function createIdentityClient(
  options?: import('./identity.account-client').IdentityClientOptions,
): IdentityClient {
  return new IdentityClient(options);
}

export function listOAuthClients(options?: RequestOptions): Promise<OAuthClient[]> {
  return identityClient.listOAuthClients(options);
}

export function revokeOAuthClient(clientId: string): Promise<void> {
  return identityClient.revokeOAuthClient(clientId);
}
