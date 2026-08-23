import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';
import { EnterpriseLayout } from './components/EnterpriseLayout';
import { createEnterpriseRoutes } from './enterprise.routes';

const rootRoute = createRootRoute({ component: Outlet });

export const enterpriseRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'enterprise',
  component: EnterpriseLayout,
});

const routeTree = rootRoute.addChildren([
  enterpriseRoute.addChildren(createEnterpriseRoutes(enterpriseRoute)),
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
