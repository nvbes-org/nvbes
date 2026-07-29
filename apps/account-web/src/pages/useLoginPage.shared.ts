import type { AccountEntry } from '@nvbes/identity-client';

import type { OAuthAuthorizeRequest } from '../identity.oauth';
import type { MfaMethod } from './LoginPage.mfa';

export const WEBAUTHN_TIMEOUT_MS = 60_000;

export function requestsMfa(
  nextStep: string | null | undefined,
  availableMethods: string[] | null | undefined,
) {
  return nextStep === 'mfa' || Boolean(availableMethods?.length);
}

export function nextAuthUser(accounts: AccountEntry[]) {
  const activeSlots = accounts
    .map((account) => parseInt(account.authuser, 10))
    .filter((slot) => !Number.isNaN(slot));

  if (activeSlots.length === 0) {
    return '0';
  }

  return String(Math.max(...activeSlots) + 1);
}

export function preferredMfaMethod(methods: string[] | null | undefined): MfaMethod | null {
  if (methods?.includes('webauthn')) {
    return 'webauthn';
  }
  if (methods?.includes('totp')) {
    return 'totp';
  }
  return methods?.includes('recovery') ? 'recovery' : null;
}

export function consentCancelUrl(request: OAuthAuthorizeRequest): string {
  const url = new URL(request.redirectUri);
  url.searchParams.set('error', 'access_denied');
  url.searchParams.set('error_description', 'The user denied consent.');
  if (request.state) {
    url.searchParams.set('state', request.state);
  }
  return url.toString();
}
