import { resetLoginFlow, submitMfaStep } from './useLoginPage.steps';
import type { LoginFormHandler, SubmitActionOptions } from './useLoginPage.actions.submit.shared';

export function buildMfaSubmitAction(
  {
    loginStateToken,
    sessionToken,
    mfaMethod,
    totpCode,
    emailCode,
    recoveryCode,
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
    await submitMfaStep({
      loginStateToken,
      sessionToken,
      mfaMethod,
      totpCode,
      emailCode,
      recoveryCode,
      submitMfa: mutations.loginMfaMutation.mutateAsync,
      startWebauthn: mutations.loginWebauthnStartMutation.mutateAsync,
      setError,
      setStep,
      setLoginStateToken,
      setSessionToken,
      finishLogin,
    });
  };
}

export function buildResetToIdentifierAction({
  setStep,
  setLoginStateToken,
  setSessionToken,
  resetMfaState,
  setError,
  setPassword,
}: SubmitActionOptions) {
  return () => {
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
