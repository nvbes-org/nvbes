import { RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';
import { useEffect } from 'react';
import AccountLayout from './components/AccountLayout';
import { createAccountRoutes, createStandaloneRoutes } from './identity.router.routes';
import NotFoundPage from './pages/NotFoundPage';

function IdentityRootLayout() {
  useEffect(() => {
    const target = accountAuthuserRedirectTarget();
    if (target) {
      window.location.replace(target);
    }
  }, []);

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

export const accountRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/account/$accountIndex',
  component: AccountLayout,
});

const standaloneRoutes = createStandaloneRoutes(rootRoute);
const accountRoutes = createAccountRoutes(accountRoute);

const routeTree = rootRoute.addChildren([
  ...standaloneRoutes,
  accountRoute.addChildren(accountRoutes),
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

function accountAuthuserRedirectTarget(): string | null {
  const { pathname, search, hash } = window.location;
  const match = pathname.match(/^\/u\/(\d{1,3})\/account(\/.*)?$/u);
  if (match) {
    const [, accountIndex, suffix = ''] = match;
    const params = new URLSearchParams(search);
    params.delete('authuser');
    const query = params.toString();
    return `/account/${accountIndex}${suffix}${query ? `?${query}` : ''}${hash}`;
  }

  const legacyAccountMatch = pathname.match(/^\/account(\/.*)?$/u);
  if (!legacyAccountMatch || /^\/account\/\d{1,3}(?:\/|$)/u.test(pathname)) {
    return null;
  }

  const [, suffix = ''] = legacyAccountMatch;
  const params = new URLSearchParams(search);
  const requestedAccountIndex = params.get('authuser');
  const accountIndex =
    requestedAccountIndex && /^\d{1,3}$/u.test(requestedAccountIndex) ? requestedAccountIndex : '0';
  params.delete('authuser');
  const query = params.toString();
  return `/account/${accountIndex}${suffix}${query ? `?${query}` : ''}${hash}`;
}
