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
import { readLoginReturnTo } from '../identity.return-to';
import { prefetchPowChallenge } from '../identity.auth.pow';
import { identityServiceBaseUrl } from '../identity.http';
import { useLoginPageActions } from './useLoginPage.actions';
import { useLoginPageBootstrap } from './useLoginPage.bootstrap';
import { useLoginPageMutations } from './useLoginPage.mutations';
import { useLoginPageState } from './useLoginPage.state';
import {
  followHostedDecision,
  loadHostedLogin,
  resumeHostedAuthorization,
} from './useUniversalLogin';
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
  const returnTo = useMemo(() => {
    const searchParams = new URLSearchParams(location.searchStr);
    return readLoginReturnTo(searchParams);
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

  const navigateToIdentity = useCallback(() => {
    if (returnTo) {
      window.location.assign(returnTo);
      return;
    }

    const accountBaseUrl = import.meta.env.VITE_ACCOUNT_WEB_BASE_URL?.trim();
    if (!accountBaseUrl) {
      state.setError('VITE_ACCOUNT_WEB_BASE_URL is required to leave Identity after login.');
      return;
    }
    window.location.assign(new URL('/profile', accountBaseUrl).toString());
  }, [returnTo, state.setError]);

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
    navigateToAccount: navigateToIdentity,
    authorizeCurrentOAuth,
  });
  const mfaWebauthnAbortRef = useRef<AbortController | null>(null);
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
    mfaWebauthnAbortRef,
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
    navigateToAccount: navigateToIdentity,
  });
  const setEmail = useCallback(
    (email: string) => {
      state.setEmail(email);
      if (email.trim()) {
        prefetchPowChallenge(identityServiceBaseUrl);
      }
    },
    [state.setEmail],
  );
  const setMfaMethod = useCallback(
    (method: import('./LoginPage.mfa').MfaMethod | null) => {
      if (method !== 'webauthn') {
        mfaWebauthnAbortRef.current?.abort();
        mfaWebauthnAbortRef.current = null;
      }
      state.setMfaMethod(method);
    },
    [state.setMfaMethod],
  );
  const conditionalWebAuthnAbortRef = useRef<AbortController | null>(null);

  const finishLoginRef = useRef(actions.finishLogin);
  useEffect(() => {
    finishLoginRef.current = actions.finishLogin;
  }, [actions.finishLogin]);

  useEffect(() => {
    if (state.checkingAuth || state.step !== 'identifier' || state.connectedAccounts.length > 0) {
      if (conditionalWebAuthnAbortRef.current) {
        conditionalWebAuthnAbortRef.current.abort();
        conditionalWebAuthnAbortRef.current = null;
      }
      return;
    }

    if (conditionalWebAuthnAbortRef.current) {
      return;
    }

    const controller = new AbortController();
    conditionalWebAuthnAbortRef.current = controller;

    void completeConditionalWebAuthnLogin({
      finishLogin: (session) => finishLoginRef.current(session),
      setSessionToken: state.setSessionToken,
      setEmail: state.setEmail,
      setLoginStateToken: state.setLoginStateToken,
      setStep: state.setStep,
      setError: state.setError,
      signal: controller.signal,
    })
      .catch(() => {
        // Ignorer l'annulation
      })
      .finally(() => {
        if (conditionalWebAuthnAbortRef.current === controller) {
          conditionalWebAuthnAbortRef.current = null;
        }
      });

    return () => {
      controller.abort();
      if (conditionalWebAuthnAbortRef.current === controller) {
        conditionalWebAuthnAbortRef.current = null;
      }
    };
  }, [
    state.checkingAuth,
    state.connectedAccounts.length,
    state.setEmail,
    state.setError,
    state.setLoginStateToken,
    state.setSessionToken,
    state.setStep,
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
    loginStateToken: state.loginStateToken,
    handlePasswordSubmit: actions.handlePasswordSubmit,
    handleUseAnotherAccount: actions.handleUseAnotherAccount,
    hasRecovery: state.hasRecovery,
    hasTotp: state.hasTotp,
    hasWebAuthn: state.hasWebAuthn,
    loading,
    location,
    isOAuthFlow: Boolean(oauthRequest || hostedStateId),
    mfaMethod: state.mfaMethod,
    oauthRequest,
    hostedConsent: hostedDecision?.kind === 'consent_required' ? hostedDecision : null,
    password: state.password,
    recoveryCode: state.recoveryCode,
    resetToIdentifier: actions.resetToIdentifier,
    sessionToken: state.sessionToken,
    setEmail,
    setError: state.setError,
    setIdentifierSubmitting: state.setIdentifierSubmitting,
    setMfaMethod,
    setPassword: state.setPassword,
    setRecoveryCode: state.setRecoveryCode,
    setTotpCode: state.setTotpCode,
    step: state.step,
    totpCode: state.totpCode,
    availableCount: state.availableCount,
  };
}
