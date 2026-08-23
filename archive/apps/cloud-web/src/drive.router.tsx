import { RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router';
import { lazy, type ReactElement, Suspense } from 'react';
import { Skeleton } from '@/components/ui/skeleton';
import { DriveRouteGate } from './DriveRouteGate';

function RouteSkeleton() {
  return <Skeleton className="min-h-svh rounded-none" />;
}

function lazyPage(loader: () => Promise<{ default: () => ReactElement }>) {
  const Component = lazy(loader);

  return function LazyPage() {
    return (
      <Suspense fallback={<RouteSkeleton />}>
        <Component />
      </Suspense>
    );
  };
}

const LazyDriveCallbackPage = lazyPage(() =>
  import('./DriveCallbackPage').then((module) => ({ default: module.DriveCallbackPage })),
);

const rootRoute = createRootRoute({
  errorComponent: RouterErrorFallback,
});

const callbackRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/callback',
  component: LazyDriveCallbackPage,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DriveRouteGate,
});

const catchAllRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '$',
  component: DriveRouteGate,
});

const routeTree = rootRoute.addChildren([callbackRoute, indexRoute, catchAllRoute]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
