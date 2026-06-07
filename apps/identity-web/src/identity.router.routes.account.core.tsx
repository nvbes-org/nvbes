import { createRoute } from '@tanstack/react-router';

import type { accountRoute } from './identity.router';
import {
  LazyAccountAuditsPage,
  LazyAccountLinkedAppsPage,
  LazyAccountNotificationsPage,
  LazyAccountPasswordPage,
  LazyAccountPersonalInfoPage,
  LazyAccountPreferencesPage,
  LazyAccountPrivacyPage,
  LazyAccountSecurityPage,
  LazyAccountSessionsPage,
  LazyAccountSocialPage,
  LazyAccountStandingPage,
  LazyAccountSubscriptionsPage,
  LazyBillingPage,
  LazyWorkspacesPage,
  LazyWorkspaceServiceAccountsPage,
} from './identity.router.pages';
import { withBilling } from './identity.router.shared';

export function createAccountCoreRoutes(account: typeof accountRoute) {
  return [
    createRoute({
      getParentRoute: () => account,
      path: '/',
      component: LazyAccountPersonalInfoPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/security',
      component: LazyAccountSecurityPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/security/password',
      component: LazyAccountPasswordPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/sessions',
      component: LazyAccountSessionsPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/privacy',
      component: LazyAccountPrivacyPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/linked-apps',
      component: LazyAccountLinkedAppsPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/audits',
      component: LazyAccountAuditsPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/workspaces',
      component: LazyWorkspacesPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/workspaces/service-accounts',
      component: LazyWorkspaceServiceAccountsPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/billing',
      component: withBilling(LazyBillingPage),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/subscriptions',
      component: withBilling(LazyAccountSubscriptionsPage),
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/notifications',
      component: LazyAccountNotificationsPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/preferences',
      component: LazyAccountPreferencesPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/standing',
      component: LazyAccountStandingPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/social',
      component: LazyAccountSocialPage,
    }),
  ];
}
