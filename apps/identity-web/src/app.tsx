import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import { useState } from 'react';
import { AuthorizationPage } from './authorization.page';
import type { AuthorizationController } from './authorization.controller';
import type { RecoveryController } from './recovery.controller';
import { RecoveryPage } from './recovery.page';
import type { LogoutController } from './logout.controller';
import { LogoutPage } from './logout.page';

export function App({
  controller,
  recovery,
  logout,
}: {
  controller: AuthorizationController;
  recovery: RecoveryController;
  logout: LogoutController;
}) {
  const [router] = useState(() => {
    const root = createRootRoute({ component: Outlet, notFoundComponent: StartPage });
    const authorization = createRoute({
      getParentRoute: () => root,
      path: '/oauth/authorize',
      component: () => <AuthorizationPage controller={controller} />,
    });
    const home = createRoute({ getParentRoute: () => root, path: '/', component: StartPage });
    const recoveryRoute = createRoute({
      getParentRoute: () => root,
      path: '/recovery',
      component: () => <RecoveryPage controller={recovery} />,
    });
    const logoutRoute = createRoute({
      getParentRoute: () => root,
      path: '/logout',
      component: () => <LogoutPage controller={logout} />,
    });
    return createRouter({
      routeTree: root.addChildren([authorization, recoveryRoute, logoutRoute, home]),
    });
  });
  return <RouterProvider router={router} />;
}

function StartPage() {
  return (
    <main className="mx-auto flex min-h-svh max-w-xl flex-col justify-center gap-6 px-6">
      <span className="text-2xl font-semibold tracking-tighter">nvbes.</span>
      <h1 className="font-heading text-4xl tracking-tight">Votre compte, vos applications.</h1>
      <p className="leading-relaxed text-muted-foreground">
        Pour vous connecter, ouvrez votre application nvbes et choisissez de vous connecter à votre
        compte.
      </p>
    </main>
  );
}
