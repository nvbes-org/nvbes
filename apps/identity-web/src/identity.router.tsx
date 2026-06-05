import { AuthErrorBoundary, BillingErrorBoundary, RouterErrorFallback } from '@nvbes/web-runtime';
import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';
import { lazy, type ReactElement, Suspense } from 'react';
import AccountLayout from './components/AccountLayout';
import NotFoundPage from './pages/NotFoundPage';

function RouteSkeleton() {
  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up">
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

function lazyPage(loader: () => Promise<{ default: () => ReactElement | null }>) {
  const Component = lazy(loader);

  return function LazyPage() {
    return (
      <Suspense fallback={<RouteSkeleton />}>
        <Component />
      </Suspense>
    );
  };
}

const LazyLoginPage = lazyPage(() => import('./pages/LoginPage'));
const LazyRegisterPage = lazyPage(() => import('./pages/RegisterPage'));
const LazyVerifyEmailPage = lazyPage(() => import('./pages/VerifyEmailPage'));
const LazyVerifyEmailResultPage = lazyPage(() => import('./pages/VerifyEmailResultPage'));
const LazyForgotPasswordPage = lazyPage(() => import('./pages/ForgotPasswordPage'));
const LazyResetPasswordPage = lazyPage(() => import('./pages/ResetPasswordPage'));
const LazyDeviceActivationPage = lazyPage(() => import('./pages/DeviceActivationPage'));
const LazyRecoveryReviewsPage = lazyPage(() => import('./pages/RecoveryReviewsPage'));
const LazyWorkerQueueStatusPage = lazyPage(() => import('./pages/WorkerQueueStatusPage'));
const LazyAccountPersonalInfoPage = lazyPage(() => import('./pages/AccountPersonalInfoPage'));
const LazyAccountSecurityPage = lazyPage(() => import('./pages/AccountSecurityPage'));
const LazyAccountPasswordPage = lazyPage(() => import('./pages/AccountPasswordPage'));
const LazyAccountSessionsPage = lazyPage(() => import('./pages/AccountSessionsPage'));
const LazyAccountPrivacyPage = lazyPage(() => import('./pages/AccountPrivacyPage'));
const LazyAccountLinkedAppsPage = lazyPage(() => import('./pages/AccountLinkedAppsPage'));
const LazyAccountAuditsPage = lazyPage(() => import('./pages/AccountAuditsPage'));
const LazyWorkspacesPage = lazyPage(() => import('./pages/WorkspacesPage'));
const LazyWorkspaceServiceAccountsPage = lazyPage(
  () => import('./pages/WorkspaceServiceAccountsPage'),
);
const LazyBillingPage = lazyPage(() => import('./pages/BillingPage'));
const LazyAccountSubscriptionsPage = lazyPage(() => import('./pages/AccountSubscriptionsPage'));
const LazyAccountNotificationsPage = lazyPage(() => import('./pages/AccountNotificationsPage'));
const LazyAccountPreferencesPage = lazyPage(() => import('./pages/AccountPreferencesPage'));
const LazyAccountStandingPage = lazyPage(() => import('./pages/AccountStandingPage'));
const LazyAccountSocialPage = lazyPage(() => import('./pages/AccountSocialPage'));
const LazyMfaPage = lazyPage(() => import('./pages/MfaPage'));
const LazyTotpSetupPage = lazyPage(() => import('./pages/TotpSetupPage'));
const LazyRecoveryCodesPage = lazyPage(() => import('./pages/RecoveryCodesPage'));
const LazyWebauthnSetupPage = lazy(() => import('./pages/WebauthnSetupPage'));

function withAuth(Component: () => ReactElement) {
  return function AuthWrapped() {
    return (
      <AuthErrorBoundary>
        <Component />
      </AuthErrorBoundary>
    );
  };
}

function withBilling(Component: () => ReactElement) {
  return function BillingWrapped() {
    return (
      <BillingErrorBoundary>
        <Component />
      </BillingErrorBoundary>
    );
  };
}

const rootRoute = createRootRoute({
  component: Outlet,
  errorComponent: RouterErrorFallback,
  notFoundComponent: NotFoundPage,
});

const standaloneRoutes = [
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/login',
    component: withAuth(LazyLoginPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/register',
    component: withAuth(LazyRegisterPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/verify',
    component: withAuth(LazyVerifyEmailPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/verify-email',
    component: withAuth(LazyVerifyEmailPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/verify-result',
    component: withAuth(LazyVerifyEmailResultPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/forgot-password',
    component: withAuth(LazyForgotPasswordPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/reset-password',
    component: withAuth(LazyResetPasswordPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/activate',
    component: withAuth(LazyDeviceActivationPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/recovery-reviews',
    component: withAuth(LazyRecoveryReviewsPage),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: '/worker-queue',
    component: LazyWorkerQueueStatusPage,
  }),
];

const accountRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/account',
  component: AccountLayout,
});

const accountRoutes = [
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/',
    component: LazyAccountPersonalInfoPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/security',
    component: LazyAccountSecurityPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/security/password',
    component: LazyAccountPasswordPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/sessions',
    component: LazyAccountSessionsPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/privacy',
    component: LazyAccountPrivacyPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/linked-apps',
    component: LazyAccountLinkedAppsPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/audits',
    component: LazyAccountAuditsPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/workspaces',
    component: LazyWorkspacesPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/workspaces/service-accounts',
    component: LazyWorkspaceServiceAccountsPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/billing',
    component: withBilling(LazyBillingPage),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/subscriptions',
    component: withBilling(LazyAccountSubscriptionsPage),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/notifications',
    component: LazyAccountNotificationsPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/preferences',
    component: LazyAccountPreferencesPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/standing',
    component: LazyAccountStandingPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/social',
    component: LazyAccountSocialPage,
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/mfa',
    component: withAuth(LazyMfaPage),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/mfa/totp/setup',
    component: withAuth(LazyTotpSetupPage),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/mfa/passkey/setup',
    component: withAuth(() => (
      <Suspense fallback={<RouteSkeleton />}>
        <LazyWebauthnSetupPage kind="passkey" />
      </Suspense>
    )),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/mfa/security-key/setup',
    component: withAuth(() => (
      <Suspense fallback={<RouteSkeleton />}>
        <LazyWebauthnSetupPage kind="security_key" />
      </Suspense>
    )),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/mfa/webauthn/setup',
    component: withAuth(() => (
      <Suspense fallback={<RouteSkeleton />}>
        <LazyWebauthnSetupPage kind="security_key" />
      </Suspense>
    )),
  }),
  createRoute({
    getParentRoute: () => accountRoute,
    path: '/mfa/recovery-codes',
    component: withAuth(LazyRecoveryCodesPage),
  }),
];

const routeTree = rootRoute.addChildren([
  ...standaloneRoutes,
  accountRoute.addChildren(accountRoutes),
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
