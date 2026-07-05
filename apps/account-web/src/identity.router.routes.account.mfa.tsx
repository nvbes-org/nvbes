import { createRoute } from '@tanstack/react-router';

import type { accountRoute } from './identity.router';
import {
  LazyEmailMfaSetupPage,
  LazyMfaPage,
  LazyRecoveryCodesPage,
  LazyTotpSetupPage,
} from './identity.router.pages';
import { renderWebauthnSetup, withAuth } from './identity.router.shared';

export function createAccountMfaRoutes(account: typeof accountRoute) {
  return [
    createRoute({
      getParentRoute: () => account,
      path: '/mfa',
      component: withAuth(LazyMfaPage),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/mfa/totp/setup',
      component: withAuth(LazyTotpSetupPage),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/mfa/email/setup',
      component: withAuth(LazyEmailMfaSetupPage),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/mfa/passkey/setup',
      component: withAuth(renderWebauthnSetup('passkey')),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/mfa/security-key/setup',
      component: withAuth(renderWebauthnSetup('security_key')),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/mfa/webauthn/setup',
      component: withAuth(renderWebauthnSetup('security_key')),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/mfa/recovery-codes',
      component: withAuth(LazyRecoveryCodesPage),
    }),
  ];
}
