import { RouterErrorFallback } from '@nvbes/web-runtime';
import {
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
  redirect,
} from '@tanstack/react-router';
import { AccountLayout } from './components/AccountLayout';
import { AccountOAuthGuard } from './components/AccountOAuthGuard';
import { ACCOUNT_WEB_PATHS } from './account.routes';
import AccountNotificationsPage from './pages/AccountNotificationsPage';
import AccountOAuthCallbackPage from './pages/AccountOAuthCallbackPage';
import AccountPreferencesPage from './pages/AccountPreferencesPage';
import AccountPrivacyPage from './pages/AccountPrivacyPage';
import AccountProfilePage from './pages/AccountProfilePage';

function RootLayout() {
  return (
    <div className="min-h-screen bg-background">
      <Outlet />
    </div>
  );
}

function ProtectedAccountLayout() {
  return (
    <AccountOAuthGuard>
      <AccountLayout />
    </AccountOAuthGuard>
  );
}

export const rootRoute = createRootRoute({
  component: RootLayout,
  errorComponent: RouterErrorFallback,
  notFoundComponent: () => (
    <main className="mx-auto flex min-h-screen max-w-lg items-center justify-center px-6 text-center">
      <div>
        <p className="text-sm font-medium text-muted-foreground">Page introuvable</p>
        <Link
          className="mt-3 inline-block text-sm font-semibold text-primary hover:underline"
          to={ACCOUNT_WEB_PATHS.profile}
        >
          Retour au compte
        </Link>
      </div>
    </main>
  ),
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: ACCOUNT_WEB_PATHS.home,
  beforeLoad: () => {
    throw redirect({ to: ACCOUNT_WEB_PATHS.profile });
  },
});

const callbackRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: ACCOUNT_WEB_PATHS.oauthCallback,
  component: AccountOAuthCallbackPage,
});

const protectedRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'account',
  component: ProtectedAccountLayout,
});

const profileRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: ACCOUNT_WEB_PATHS.profile,
  component: AccountProfilePage,
});

const preferencesRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: ACCOUNT_WEB_PATHS.preferences,
  component: AccountPreferencesPage,
});

const notificationsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: ACCOUNT_WEB_PATHS.notifications,
  component: AccountNotificationsPage,
});

const privacyRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: ACCOUNT_WEB_PATHS.privacy,
  component: AccountPrivacyPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  callbackRoute,
  protectedRoute.addChildren([profileRoute, preferencesRoute, notificationsRoute, privacyRoute]),
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
