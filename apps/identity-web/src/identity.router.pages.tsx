import { lazyPage } from './identity.router.shared';

export const LazyLoginPage = lazyPage(() => import('./pages/LoginPage'));
export const LazyRegisterPage = lazyPage(() => import('./pages/RegisterPage'));
export const LazyVerifyEmailPage = lazyPage(() => import('./pages/VerifyEmailPage'));
export const LazyVerifyEmailResultPage = lazyPage(() => import('./pages/VerifyEmailResultPage'));
export const LazyForgotPasswordPage = lazyPage(() => import('./pages/ForgotPasswordPage'));
export const LazyResetPasswordPage = lazyPage(() => import('./pages/ResetPasswordPage'));
export const LazyIdentitySecurityPage = lazyPage(() => import('./pages/IdentitySecurityPage'));
export const LazyIdentityPasswordPage = lazyPage(() => import('./pages/IdentityPasswordPage'));
export const LazyIdentitySessionsPage = lazyPage(() => import('./pages/IdentitySessionsPage'));
export const LazyIdentityLinkedAppsPage = lazyPage(() => import('./pages/IdentityLinkedAppsPage'));
export const LazyIdentityEmailAddressesPage = lazyPage(
  () => import('./pages/IdentityEmailAddresses'),
);
export const LazyMfaPage = lazyPage(() => import('./pages/MfaPage'));
export const LazyTotpSetupPage = lazyPage(() => import('./pages/TotpSetupPage'));
export const LazyRecoveryCodesPage = lazyPage(() => import('./pages/RecoveryCodesPage'));
