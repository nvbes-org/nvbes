import { createRoute } from '@tanstack/react-router';
import type { enterpriseRoute } from './enterprise.router';
import {
  AccessReviewsRoutePage,
  AuditLogsRoutePage,
  BillingRoutePage,
  DevelopersRoutePage,
  OverviewRoutePage,
  PoliciesRoutePage,
  SecurityRoutePage,
  SettingsRoutePage,
  UsageRoutePage,
  WorkspacesRoutePage,
} from './enterprise.router.pages';
import { UsersPage } from './pages/UsersPage';

export function createEnterpriseRoutes(enterprise: typeof enterpriseRoute) {
  return [
    createRoute({
      getParentRoute: () => enterprise,
      path: '/',
      component: OverviewRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/users',
      component: UsersPage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/workspaces',
      component: WorkspacesRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/developers',
      component: DevelopersRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/policies',
      component: PoliciesRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/security',
      component: SecurityRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/access-reviews',
      component: AccessReviewsRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/audit-logs',
      component: AuditLogsRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/billing',
      component: BillingRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/usage',
      component: UsageRoutePage,
    }),
    createRoute({
      getParentRoute: () => enterprise,
      path: '/settings',
      component: SettingsRoutePage,
    }),
  ];
}
