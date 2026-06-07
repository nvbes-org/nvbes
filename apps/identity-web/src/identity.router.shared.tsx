import { AuthErrorBoundary, BillingErrorBoundary } from '@nvbes/web-runtime';
import { lazy, type ReactElement, Suspense } from 'react';

export function RouteSkeleton() {
  return (
    <div className="flex animate-fade-slide-up flex-col gap-6">
      <div>
        <div className="h-6 w-48 rounded bg-muted" />
        <div className="mt-2 h-4 w-80 rounded bg-muted" />
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        {Array.from({ length: 4 }).map((_, index) => (
          <div key={index} className="h-40 rounded-3xl bg-muted" />
        ))}
      </div>
    </div>
  );
}

export function lazyPage(loader: () => Promise<{ default: () => ReactElement | null }>) {
  const Component = lazy(loader);

  return function LazyPage() {
    return (
      <Suspense fallback={<RouteSkeleton />}>
        <Component />
      </Suspense>
    );
  };
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

export function withBilling(Component: () => ReactElement) {
  return function BillingWrapped() {
    return (
      <BillingErrorBoundary>
        <Component />
      </BillingErrorBoundary>
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
