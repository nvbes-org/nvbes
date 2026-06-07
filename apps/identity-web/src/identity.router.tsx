import { RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';
import AccountLayout from './components/AccountLayout';
import { createAccountRoutes, createStandaloneRoutes } from './identity.router.routes';
import NotFoundPage from './pages/NotFoundPage';

export const rootRoute = createRootRoute({
  component: Outlet,
  errorComponent: RouterErrorFallback,
  notFoundComponent: NotFoundPage,
});

export const accountRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/account',
  component: AccountLayout,
});

const standaloneRoutes = createStandaloneRoutes(rootRoute);
const accountRoutes = createAccountRoutes(accountRoute);

const routeTree = rootRoute.addChildren([
  ...standaloneRoutes,
  accountRoute.addChildren(accountRoutes),
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
