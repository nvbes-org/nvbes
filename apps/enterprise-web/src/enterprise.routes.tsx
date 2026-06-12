import { createRoute } from '@tanstack/react-router';
import type { enterpriseRoute } from './enterprise.router';
import {
  AuditLogsRoutePage,
  BillingRoutePage,
  DevelopersRoutePage,
  OverviewRoutePage,
  PoliciesRoutePage,
  SecurityRoutePage,
  SettingsRoutePage,
  UsageRoutePage,
  UsersRoutePage,
  WorkspacesRoutePage,
} from './enterprise.router.pages';

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
      component: UsersRoutePage,
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
