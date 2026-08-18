import { resetLoginFlow, submitMfaStep } from './useLoginPage.steps';
import type { LoginFormHandler, SubmitActionOptions } from './useLoginPage.actions.submit.shared';

export function buildMfaSubmitAction(
  {
    loginStateToken,
    sessionToken,
    mfaMethod,
    totpCode,
    recoveryCode,
    mfaWebauthnAbortRef,
    mutations,
    setError,
    setStep,
    setLoginStateToken,
    setSessionToken,
  }: SubmitActionOptions,
  finishLogin: (session: string | null) => Promise<void>,
): LoginFormHandler {
  return async (event) => {
    event.preventDefault();
    const controller = mfaMethod === 'webauthn' ? new AbortController() : null;
    mfaWebauthnAbortRef.current?.abort();
    mfaWebauthnAbortRef.current = controller;
    try {
      await submitMfaStep({
        loginStateToken,
        sessionToken,
        mfaMethod,
        totpCode,
        recoveryCode,
        webauthnSignal: controller?.signal,
        submitMfa: mutations.loginMfaMutation.mutateAsync,
        startWebauthn: mutations.loginWebauthnStartMutation.mutateAsync,
        setError,
        setStep,
        setLoginStateToken,
        setSessionToken,
        finishLogin,
      });
    } finally {
      if (mfaWebauthnAbortRef.current === controller) {
        mfaWebauthnAbortRef.current = null;
      }
    }
  };
}

export function buildResetToIdentifierAction({
  setStep,
  setLoginStateToken,
  setSessionToken,
  resetMfaState,
  setError,
  setPassword,
  mfaWebauthnAbortRef,
}: SubmitActionOptions) {
  return () => {
    mfaWebauthnAbortRef.current?.abort();
    mfaWebauthnAbortRef.current = null;
    resetLoginFlow({
      setStep,
      setLoginStateToken,
      setSessionToken,
      resetMfaState,
      setError,
      setPassword,
    });
  };
}
