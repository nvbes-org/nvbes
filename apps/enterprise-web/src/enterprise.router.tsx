import { RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRouter } from '@tanstack/react-router';

// Temporary Task 1 bootstrap route tree. Task 2 owns the real enterprise routes.
const rootRoute = createRootRoute({
  component: () => null,
  errorComponent: RouterErrorFallback,
});

const routeTree = rootRoute;

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
