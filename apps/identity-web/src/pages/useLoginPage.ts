import { type AccountEntry } from '@nvbes/identity-client';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { readHostedStateId, type HostedLoginDecision } from '../identity.universal-login.api';
import {
  authorizeIdentitySession,
  clearPendingOAuthAuthorizeRequest,
  readOAuthAuthorizeRequest,
  readPendingOAuthAuthorizeRequest,
} from '../identity.oauth';
import { useLoginPageActions } from './useLoginPage.actions';
import { useLoginPageBootstrap } from './useLoginPage.bootstrap';
import { useLoginPageMutations } from './useLoginPage.mutations';
import { useLoginPageState } from './useLoginPage.state';
import { followHostedDecision, loadHostedLogin, resumeHostedAuthorization } from './useUniversalLogin';
import { completeConditionalWebAuthnLogin } from './useLoginPage.webauthn.conditional';

export function useLoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const oauthRequest = useMemo(() => {
    const searchParams = new URLSearchParams(location.searchStr);
    return readOAuthAuthorizeRequest(searchParams) ?? readPendingOAuthAuthorizeRequest();
  }, [location.searchStr]);
  const hostedStateId = useMemo(() => {
    const searchParams = new URLSearchParams(location.searchStr);
    return readHostedStateId(searchParams);
  }, [location.searchStr]);
  const state = useLoginPageState();
  const [hostedDecision, setHostedDecision] = useState<HostedLoginDecision | null>(null);

  const {
    loginIdentifierMutation,
    loginPasswordMutation,
    loginMfaMutation,
    loginWebauthnStartMutation,
    loading,
  } = useLoginPageMutations();

  const navigateToAccount = useCallback(() => {
    void navigate({ to: '/account' });
  }, [navigate]);

  const handleHostedDecision = useCallback(
    (decision: HostedLoginDecision) => {
      setHostedDecision(decision);
      if (decision.kind === 'consent_required') {
        state.setStep('consent');
        return;
      }
      if (decision.kind === 'error_page') {
        state.setError(decision.message);
        return;
      }
      if (decision.kind === 'login_required') {
        state.setStep('identifier');
        return;
      }
      followHostedDecision(decision);
    },
    [state.setError, state.setStep],
  );

  const authorizeCurrentOAuth = useCallback(async () => {
    if (hostedStateId) {
      const decision = await resumeHostedAuthorization(hostedStateId);
      handleHostedDecision(decision);
      return;
    }

    if (!oauthRequest) {
      return;
    }
    await authorizeIdentitySession(null, oauthRequest);
    clearPendingOAuthAuthorizeRequest();
  }, [handleHostedDecision, hostedStateId, oauthRequest]);

  useEffect(() => {
    if (!hostedStateId) {
      setHostedDecision(null);
      return;
    }

    void loadHostedLogin(hostedStateId)
      .then(handleHostedDecision)
      .catch((err) => {
        state.setError(err instanceof Error ? err.message : 'Login request failed.');
      });
  }, [handleHostedDecision, hostedStateId, state.setError]);

  const decoyRef = useLoginPageBootstrap({
    checkingAuth: state.checkingAuth,
    locationSearchStr: location.searchStr,
    hasOAuthRequest: Boolean(oauthRequest || hostedStateId),
    setCheckingAuth: state.setCheckingAuth,
    setConnectedAccounts: state.setConnectedAccounts as (value: AccountEntry[]) => void,
    setStep: state.setStep,
    setEmail: state.setEmail,
    setPassword: state.setPassword,
    setError: state.setError,
    navigateToAccount,
    authorizeCurrentOAuth,
  });
  const actions = useLoginPageActions({
    navigate,
    oauthRequest,
    hostedStateId,
    onHostedDecision: handleHostedDecision,
    connectedAccounts: state.connectedAccounts,
    setConnectedAccounts: state.setConnectedAccounts,
    email: state.email,
    password: state.password,
    loginStateToken: state.loginStateToken,
    sessionToken: state.sessionToken,
    mfaMethod: state.mfaMethod,
    totpCode: state.totpCode,
    recoveryCode: state.recoveryCode,
    decoyRef,
    mutations: {
      loginIdentifierMutation,
      loginPasswordMutation,
      loginMfaMutation,
      loginWebauthnStartMutation,
    },
    resetMfaState: state.resetMfaState,
    setCheckingAuth: state.setCheckingAuth,
    setStep: state.setStep,
    setEmail: state.setEmail,
    setPassword: state.setPassword,
    setIdentifierSubmitting: state.setIdentifierSubmitting,
    setLoginStateToken: state.setLoginStateToken,
    setSessionToken: state.setSessionToken,
    setAvailableMethods: state.setAvailableMethods,
    setMfaMethod: state.setMfaMethod,
    setError: state.setError,
    navigateToAccount,
  });
  const conditionalWebAuthnStartedRef = useRef(false);

  useEffect(() => {
    if (
      state.checkingAuth ||
      state.step !== 'identifier' ||
      state.connectedAccounts.length > 0 ||
      conditionalWebAuthnStartedRef.current
    ) {
      return;
    }

    conditionalWebAuthnStartedRef.current = true;
    void completeConditionalWebAuthnLogin({
      finishLogin: actions.finishLogin,
      setSessionToken: state.setSessionToken,
      setError: state.setError,
    }).catch(() => {
      conditionalWebAuthnStartedRef.current = false;
    });
  }, [
    actions.finishLogin,
    state.checkingAuth,
    state.connectedAccounts.length,
    state.setError,
    state.setSessionToken,
    state.step,
  ]);

  return {
    checkingAuth: state.checkingAuth,
    connectedAccounts: state.connectedAccounts,
    email: state.email,
    error: state.error,
    identifierSubmitting: state.identifierSubmitting,
    handleAccountSelect: actions.handleAccountSelect,
    handleDisconnectAccount: actions.handleDisconnectAccount,
    handleDisconnectAllAccounts: actions.handleDisconnectAllAccounts,
    handleConsentApprove: actions.handleConsentApprove,
    handleConsentCancel: actions.handleConsentCancel,
    handleIdentifierSubmit: actions.handleIdentifierSubmit,
    handleMfaSubmit: actions.handleMfaSubmit,
    handlePasswordSubmit: actions.handlePasswordSubmit,
    handleUseAnotherAccount: actions.handleUseAnotherAccount,
    hasRecovery: state.hasRecovery,
    hasTotp: state.hasTotp,
    hasWebAuthn: state.hasWebAuthn,
    loading,
    location,
    mfaMethod: state.mfaMethod,
    oauthRequest,
    hostedConsent: hostedDecision?.kind === 'consent_required' ? hostedDecision : null,
    password: state.password,
    recoveryCode: state.recoveryCode,
    resetToIdentifier: actions.resetToIdentifier,
    sessionToken: state.sessionToken,
    setEmail: state.setEmail,
    setError: state.setError,
    setIdentifierSubmitting: state.setIdentifierSubmitting,
    setMfaMethod: state.setMfaMethod,
    setPassword: state.setPassword,
    setRecoveryCode: state.setRecoveryCode,
    setTotpCode: state.setTotpCode,
    step: state.step,
    totpCode: state.totpCode,
    availableCount: state.availableCount,
  };
}
