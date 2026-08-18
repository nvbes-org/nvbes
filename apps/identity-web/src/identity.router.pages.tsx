import { lazyPage } from './identity.router.shared';

export const LazyLoginPage = lazyPage(() => import('./pages/LoginPage'));
export const LazyRegisterPage = lazyPage(() => import('./pages/RegisterPage'));
export const LazyVerifyEmailPage = lazyPage(() => import('./pages/VerifyEmailPage'));
export const LazyVerifyEmailResultPage = lazyPage(() => import('./pages/VerifyEmailResultPage'));
