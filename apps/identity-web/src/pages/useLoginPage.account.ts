import type { AccountEntry } from '@nvbes/identity-client';
import type { UseNavigateResult } from '@tanstack/react-router';

import type { LoginStep } from './LoginProgress';
import { consentCancelUrl, nextAuthUser } from './useLoginPage.shared';
import type { OAuthAuthorizeRequest } from '../identity.oauth';

export function selectAccount(authuser: string) {
  const url = new URL(window.location.href);
  url.searchParams.set('authuser', authuser);
  window.location.href = url.toString();
}

type NavigateFn = UseNavigateResult<string>;

export async function useAnotherAccount({
  navigate,
  connectedAccounts,
  setError,
  setEmail,
  setPassword,
  setStep,
}: {
  navigate: NavigateFn;
  connectedAccounts: AccountEntry[];
  setError: (value: string | null) => void;
  setEmail: (value: string) => void;
  setPassword: (value: string) => void;
  setStep: (value: LoginStep) => void;
}) {
  setError(null);

  const authuser = nextAuthUser(connectedAccounts);

  await (
    navigate as (opts: {
      search: (prev: Record<string, unknown>) => Record<string, unknown>;
    }) => Promise<void>
  )({
    search: (prev: Record<string, unknown>) => ({
      ...prev,
      authuser,
    }),
  });

  const url = new URL(window.location.href);
  url.searchParams.set('authuser', authuser);
  window.history.replaceState(null, '', url.toString());

  setEmail('');
  setPassword('');
  setStep('identifier');
}

export function cancelConsent({
  oauthRequest,
  clearPendingOAuthAuthorizeRequest,
  navigateToAccount,
}: {
  oauthRequest: OAuthAuthorizeRequest | null;
  clearPendingOAuthAuthorizeRequest: () => void;
  navigateToAccount: () => void;
}) {
  if (oauthRequest) {
    clearPendingOAuthAuthorizeRequest();
    window.location.assign(consentCancelUrl(oauthRequest));
    return;
  }

  navigateToAccount();
}
