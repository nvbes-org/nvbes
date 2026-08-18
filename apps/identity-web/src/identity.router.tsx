import { RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRouter, Outlet } from '@tanstack/react-router';

import { createStandaloneRoutes } from './identity.router.routes';
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

const standaloneRoutes = createStandaloneRoutes(rootRoute);
const routeTree = rootRoute.addChildren(standaloneRoutes);

export const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
