import { useCallback } from 'react';
import { storePasswordCredential } from '@nvbes/identity-sdk-web';

import { clearPendingOAuthAuthorizeRequest } from '../identity.oauth';
import { cancelConsent, selectAccount, useAnotherAccount } from './useLoginPage.account';
import { approveConsent, finishLoginSession } from './useLoginPage.oauth';
import type { UseLoginPageActionsOptions } from './useLoginPage.actions.shared';

export function useLoginPageAccountActions({
  navigate,
  oauthRequest,
  connectedAccounts,
  email,
  password,
  sessionToken,
  setCheckingAuth,
  setStep,
  setEmail,
  setPassword,
  setSessionToken,
  setError,
  navigateToAccount,
}: Pick<
  UseLoginPageActionsOptions,
  | 'navigate'
  | 'oauthRequest'
  | 'connectedAccounts'
  | 'email'
  | 'password'
  | 'sessionToken'
  | 'setCheckingAuth'
  | 'setStep'
  | 'setEmail'
  | 'setPassword'
  | 'setSessionToken'
  | 'setError'
  | 'navigateToAccount'
>) {
  const finishLogin = useCallback(
    async (session: string | null) => {
      await finishLoginSession({
        email,
        password,
        sessionToken: session,
        oauthRequest,
        storePasswordCredential,
        navigateToAccount,
        onConsentRequired: (nextSessionToken) => {
          setSessionToken(nextSessionToken);
          setStep('consent');
        },
        onInvalidSession: () => {
          setSessionToken(null);
          setStep('identifier');
        },
      });
    },
    [email, password, oauthRequest, navigateToAccount, setSessionToken, setStep],
  );

  const handleAccountSelect = (authuser: string) => {
    selectAccount(authuser);
  };

  const handleUseAnotherAccount = async () => {
    await useAnotherAccount({
      navigate,
      connectedAccounts,
      setError,
      setEmail,
      setPassword,
      setStep,
    });
  };

  const handleConsentApprove = async () => {
    await approveConsent({
      oauthRequest,
      sessionToken,
      setError,
      setCheckingAuth,
      setSessionToken,
      setStep,
    });
  };

  const handleConsentCancel = () => {
    cancelConsent({
      oauthRequest,
      clearPendingOAuthAuthorizeRequest,
      navigateToAccount,
    });
  };

  return {
    finishLogin,
    handleAccountSelect,
    handleUseAnotherAccount,
    handleConsentApprove,
    handleConsentCancel,
  };
}
