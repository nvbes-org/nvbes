import { RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';

import IdentityLayout from './components/IdentityLayout';
import { createIdentityRoutes, createStandaloneRoutes } from './identity.router.routes';
import NotFoundPage from './pages/NotFoundPage';

function IdentityRootLayout() {
  return (
    <div className="min-h-screen bg-background">
      <Outlet />
    </div>
  );
}

export const rootRoute = createRootRoute({
  component: IdentityRootLayout,
  errorComponent: RouterErrorFallback,
  notFoundComponent: NotFoundPage,
});

export const identityRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'identity',
  component: IdentityLayout,
});

const standaloneRoutes = createStandaloneRoutes(rootRoute);
const identityRoutes = createIdentityRoutes(identityRoute);
const routeTree = rootRoute.addChildren([
  ...standaloneRoutes,
  identityRoute.addChildren(identityRoutes),
]);

export const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
