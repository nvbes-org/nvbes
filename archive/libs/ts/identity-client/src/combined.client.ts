import type { IdentityClientOptions } from './identity.account-client';
import { IdentityClient } from './identity.enterprise-client';

export const identityClient = new IdentityClient();

export function createIdentityClient(options?: IdentityClientOptions): IdentityClient {
  return new IdentityClient(options);
}
