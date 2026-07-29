import { detectSessionStale } from '@nvbes/web-runtime';

export type AccountAuthenticationDisposition = 'none' | 'reauthenticate' | 'redirect-login';

export function accountAuthenticationDisposition(error: unknown): AccountAuthenticationDisposition {
  const session = detectSessionStale(error);
  if (session.stale) {
    return 'reauthenticate';
  }
  return session.status === 401 ? 'redirect-login' : 'none';
}
