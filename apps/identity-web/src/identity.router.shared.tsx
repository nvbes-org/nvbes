import { AuthErrorBoundary } from '@nvbes/web-runtime';
import { lazyRouteComponent } from '@tanstack/react-router';
import { lazy, type ReactElement, Suspense } from 'react';
import { Skeleton } from '@/components/ui/skeleton';

export function RouteSkeleton() {
  return (
    <div className="flex animate-fade-slide-up flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-48" />
        <Skeleton className="mt-2 h-4 w-80" />
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        {Array.from({ length: 4 }).map((_, index) => (
          <Skeleton key={index} className="h-40 rounded-3xl" />
        ))}
      </div>
    </div>
  );
}

export function lazyPage(loader: () => Promise<{ default: () => ReactElement | null }>) {
  const Component = lazyRouteComponent(loader);

  function LazyPage() {
    return (
      <Suspense fallback={<RouteSkeleton />}>
        <Component />
      </Suspense>
    );
  }

  LazyPage.preload = () => Component.preload?.() ?? Promise.resolve();
  return LazyPage;
}

export function withAuth(Component: () => ReactElement) {
  return function AuthWrapped() {
    return (
      <AuthErrorBoundary>
        <Component />
      </AuthErrorBoundary>
    );
  };
}

const LazyWebauthnSetupPage = lazy(() => import('./pages/WebauthnSetupPage'));

export function renderWebauthnSetup(kind: 'passkey' | 'security_key') {
  return function WebauthnSetupRoute() {
    return (
      <Suspense fallback={<RouteSkeleton />}>
        <LazyWebauthnSetupPage kind={kind} />
      </Suspense>
    );
  };
}
