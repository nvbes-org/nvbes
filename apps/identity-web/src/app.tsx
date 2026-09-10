import { AuthShell } from './auth.shell';
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
import { PasswordRecoveryPage } from './password-recovery.page';
import type { PasswordRecoveryController } from './password-recovery.controller';

export function App({
  controller,
  recovery,
  logout,
  passwordRecovery,
}: {
  controller: AuthorizationController;
  recovery: RecoveryController;
  logout: LogoutController;
  passwordRecovery: PasswordRecoveryController;
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
    const passwordRecoveryRoute = createRoute({
      getParentRoute: () => root,
      path: '/password-recovery',
      component: () => <PasswordRecoveryPage controller={passwordRecovery} />,
    });
    return createRouter({
      routeTree: root.addChildren([
        authorization,
        recoveryRoute,
        logoutRoute,
        passwordRecoveryRoute,
        home,
      ]),
    });
  });
  return <RouterProvider router={router} />;
}

function StartPage() {
  return (
    <AuthShell title="Votre compte, vos applications.">
      <p className="leading-relaxed text-muted-foreground">
        Pour vous connecter, ouvrez votre application nvbes et choisissez de vous connecter à votre
        compte.
      </p>
    </AuthShell>
  );
}
