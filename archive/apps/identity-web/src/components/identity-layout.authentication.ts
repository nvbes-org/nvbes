import { detectSessionStale } from '@nvbes/web-runtime';

export type IdentityAuthenticationDisposition = 'none' | 'reauthenticate' | 'redirect-login';

export function identityAuthenticationDisposition(
  error: unknown,
): IdentityAuthenticationDisposition {
  const session = detectSessionStale(error);
  if (session.stale) {
    return 'reauthenticate';
  }
  return session.status === 401 ? 'redirect-login' : 'none';
}
