import { useCallback } from 'react';
import { identityClient } from '@nvbes/identity-client';
import { storePasswordCredential } from '@nvbes/identity-sdk-web';
import { clientErrorMessage } from '@nvbes/web-runtime';

import { clearPendingOAuthAuthorizeRequest } from '../identity.oauth';
import { cancelConsent, selectAccount, useAnotherAccount } from './useLoginPage.account';
import { approveConsent, finishLoginSession } from './useLoginPage.oauth';
import type { UseLoginPageActionsOptions } from './useLoginPage.actions.shared';
import {
  approveUniversalLoginConsent,
  denyUniversalLoginConsent,
  resumeHostedAuthorization,
} from './useUniversalLogin';

export function useLoginPageAccountActions({
  navigate,
  oauthRequest,
  hostedStateId,
  onHostedDecision,
  connectedAccounts,
  setConnectedAccounts,
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
  | 'hostedStateId'
  | 'onHostedDecision'
  | 'connectedAccounts'
  | 'setConnectedAccounts'
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
      if (hostedStateId) {
        const decision = await resumeHostedAuthorization(hostedStateId);
        onHostedDecision(decision);
        return;
      }

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
    [
      email,
      hostedStateId,
      onHostedDecision,
      password,
      oauthRequest,
      navigateToAccount,
      setSessionToken,
      setStep,
    ],
  );

  const handleAccountSelect = (authuser: string) => {
    const account = connectedAccounts.find((entry) => entry.authuser === authuser);
    if (account?.status === 'expired') {
      const url = new URL(window.location.href);
      url.searchParams.set('authuser', authuser);
      window.history.replaceState(null, '', url.toString());
      setEmail(account.user.email);
      setPassword('');
      setError(account.message ?? 'Session expirée, veuillez vous reconnecter.');
      setStep('identifier');
      return;
    }

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

  const handleDisconnectAccount = async (authuser: string) => {
    const account = connectedAccounts.find((entry) => entry.authuser === authuser);
    if (!account) {
      return;
    }

    try {
      setError(null);
      if (account.status !== 'expired') {
        try {
          await identityClient.revokeSession(account.session.id);
        } catch {
          // Ignore revocation errors (e.g. session already expired or unauthorized)
        }
      }
      await identityClient.forgetAccount(account.authuser);
      setConnectedAccounts((previous) => previous.filter((entry) => entry.authuser !== authuser));
      if (connectedAccounts.length <= 1) {
        setStep('identifier');
      }
    } catch (err) {
      setError(clientErrorMessage(err, 'Impossible de deconnecter ce compte'));
    }
  };

  const handleDisconnectAllAccounts = async () => {
    const sessionsToRevoke = [...connectedAccounts].sort((left, right) => {
      if (left.session.current === right.session.current) {
        return 0;
      }
      return left.session.current ? 1 : -1;
    });

    try {
      setError(null);
      for (const account of sessionsToRevoke) {
        if (account.status !== 'expired') {
          try {
            await identityClient.revokeSession(account.session.id);
          } catch {
            // Ignore revocation errors
          }
        }
        await identityClient.forgetAccount(account.authuser);
      }
      setConnectedAccounts([]);
      setStep('identifier');
    } catch (err) {
      setError(clientErrorMessage(err, 'Impossible de deconnecter tous les comptes'));
    }
  };

  const handleConsentApprove = async () => {
    if (hostedStateId) {
      try {
        setError(null);
        setCheckingAuth(true);
        const decision = await approveUniversalLoginConsent(hostedStateId);
        onHostedDecision(decision);
      } catch (err) {
        setError(clientErrorMessage(err, 'Failed to grant consent'));
      } finally {
        setCheckingAuth(false);
      }
      return;
    }

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
    if (hostedStateId) {
      void denyUniversalLoginConsent(hostedStateId)
        .then(onHostedDecision)
        .catch((err) => {
          setError(clientErrorMessage(err, 'Failed to deny consent'));
        });
      return;
    }

    cancelConsent({
      oauthRequest,
      clearPendingOAuthAuthorizeRequest,
      navigateToAccount,
    });
  };

  return {
    finishLogin,
    handleAccountSelect,
    handleDisconnectAccount,
    handleDisconnectAllAccounts,
    handleUseAnotherAccount,
    handleConsentApprove,
    handleConsentCancel,
  };
}
