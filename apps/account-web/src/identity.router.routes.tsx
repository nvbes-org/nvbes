import type { accountRoute } from './identity.router';
import { createAccountCoreRoutes } from './identity.router.routes.account.core';
import { createAccountMfaRoutes } from './identity.router.routes.account.mfa';
export { createStandaloneRoutes } from './identity.router.routes.standalone';

export function createAccountRoutes(account: typeof accountRoute) {
  return [...createAccountCoreRoutes(account), ...createAccountMfaRoutes(account)];
}
