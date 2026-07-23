import { createRoute } from '@tanstack/react-router';

import type { accountRoute } from './identity.router';
import {
  LazyAccountLinkedAppsPage,
  LazyAccountNotificationsPage,
  LazyAccountPasswordPage,
  LazyAccountPersonalInfoPage,
  LazyAccountPreferencesPage,
  LazyAccountPrivacyPage,
  LazyAccountSecurityPage,
  LazyAccountSessionsPage,
} from './identity.router.pages';

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
      path: '/notifications',
      component: LazyAccountNotificationsPage,
    }),
    createRoute({
      getParentRoute: () => account,
      path: '/preferences',
      component: LazyAccountPreferencesPage,
    }),
  ];
}
