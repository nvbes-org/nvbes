import { createRoute, redirect } from '@tanstack/react-router';

import type { rootRoute } from './identity.router';
import {
  LazyLoginPage,
  LazyRegisterPage,
  LazyVerifyEmailPage,
  LazyVerifyEmailResultPage,
} from './identity.router.pages';
import { withAuth } from './identity.router.shared';

export function createStandaloneRoutes(root: typeof rootRoute) {
  return [
    createRoute({
      getParentRoute: () => root,
      path: '/',
      beforeLoad: () => {
        throw redirect({ to: '/login' });
      },
    }),
    createRoute({
      getParentRoute: () => root,
      path: '/login',
      component: withAuth(LazyLoginPage),
    }),
    createRoute({
      getParentRoute: () => root,
      path: '/register',
      component: withAuth(LazyRegisterPage),
    }),
    createRoute({
      getParentRoute: () => root,
      path: '/verify',
      component: withAuth(LazyVerifyEmailPage),
    }),
    createRoute({
      getParentRoute: () => root,
      path: '/verify-email',
      component: withAuth(LazyVerifyEmailPage),
    }),
    createRoute({
      getParentRoute: () => root,
      path: '/verify-result',
      component: withAuth(LazyVerifyEmailResultPage),
    }),
  ];
}
