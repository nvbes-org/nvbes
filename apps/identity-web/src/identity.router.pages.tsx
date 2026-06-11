import { lazyPage } from './identity.router.shared';

export const LazyLoginPage = lazyPage(() => import('./pages/LoginPage'));
export const LazyRegisterPage = lazyPage(() => import('./pages/RegisterPage'));
export const LazyVerifyEmailPage = lazyPage(() => import('./pages/VerifyEmailPage'));
export const LazyVerifyEmailResultPage = lazyPage(() => import('./pages/VerifyEmailResultPage'));
export const LazyForgotPasswordPage = lazyPage(() => import('./pages/ForgotPasswordPage'));
export const LazyResetPasswordPage = lazyPage(() => import('./pages/ResetPasswordPage'));
export const LazyDeviceActivationPage = lazyPage(() => import('./pages/DeviceActivationPage'));
export const LazyRecoveryReviewsPage = lazyPage(() => import('./pages/RecoveryReviewsPage'));
export const LazyWorkerQueueStatusPage = lazyPage(() => import('./pages/WorkerQueueStatusPage'));
export const LazyAccountPersonalInfoPage = lazyPage(
  () => import('./pages/AccountPersonalInfoPage'),
);
export const LazyAccountSecurityPage = lazyPage(() => import('./pages/AccountSecurityPage'));
export const LazyAccountPasswordPage = lazyPage(() => import('./pages/AccountPasswordPage'));
export const LazyAccountSessionsPage = lazyPage(() => import('./pages/AccountSessionsPage'));
export const LazyAccountPrivacyPage = lazyPage(() => import('./pages/AccountPrivacyPage'));
export const LazyAccountLinkedAppsPage = lazyPage(() => import('./pages/AccountLinkedAppsPage'));
export const LazyAccountAuditsPage = lazyPage(() => import('./pages/AccountAuditsPage'));
export const LazyWorkspacesPage = lazyPage(() => import('./pages/WorkspacesPage'));
export const LazyWorkspaceServiceAccountsPage = lazyPage(
  () => import('./pages/WorkspaceServiceAccountsPage'),
);
export const LazyBillingPage = lazyPage(() => import('./pages/BillingPage'));
export const LazyAccountSubscriptionsPage = lazyPage(
  () => import('./pages/AccountSubscriptionsPage'),
);
export const LazyAccountNotificationsPage = lazyPage(
  () => import('./pages/AccountNotificationsPage'),
);
export const LazyAccountPreferencesPage = lazyPage(() => import('./pages/AccountPreferencesPage'));
export const LazyAccountSocialPage = lazyPage(() => import('./pages/AccountSocialPage'));
export const LazyMfaPage = lazyPage(() => import('./pages/MfaPage'));
export const LazyTotpSetupPage = lazyPage(() => import('./pages/TotpSetupPage'));
export const LazyRecoveryCodesPage = lazyPage(() => import('./pages/RecoveryCodesPage'));
