import { createRoute } from '@tanstack/react-router';

import type { identityRoute } from './identity.router';
import {
  LazyIdentityEmailAddressesPage,
  LazyIdentityLinkedAppsPage,
  LazyIdentityPasswordPage,
  LazyIdentitySecurityPage,
  LazyIdentitySessionsPage,
  LazyMfaPage,
  LazyRecoveryCodesPage,
  LazyTotpSetupPage,
} from './identity.router.pages';
import { renderWebauthnSetup, withAuth } from './identity.router.shared';

export function createIdentityRoutes(identity: typeof identityRoute) {
  return [
    createRoute({
      getParentRoute: () => identity,
      path: '/security',
      component: LazyIdentitySecurityPage,
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/security/password',
      component: LazyIdentityPasswordPage,
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/email-addresses',
      component: LazyIdentityEmailAddressesPage,
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/sessions',
      component: LazyIdentitySessionsPage,
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/linked-apps',
      component: LazyIdentityLinkedAppsPage,
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/mfa',
      component: withAuth(LazyMfaPage),
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/mfa/totp/setup',
      component: withAuth(LazyTotpSetupPage),
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/mfa/passkey/setup',
      component: withAuth(renderWebauthnSetup('passkey')),
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/mfa/security-key/setup',
      component: withAuth(renderWebauthnSetup('security_key')),
    }),
    createRoute({
      getParentRoute: () => identity,
      path: '/mfa/recovery-codes',
      component: withAuth(LazyRecoveryCodesPage),
    }),
  ];
}
