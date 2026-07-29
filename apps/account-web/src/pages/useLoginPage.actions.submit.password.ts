import { submitPasswordStep } from './useLoginPage.steps';
import type { LoginFormHandler, SubmitActionOptions } from './useLoginPage.actions.submit.shared';

export function buildPasswordSubmitAction(
  {
    navigate,
    email,
    password,
    loginStateToken,
    mutations,
    setError,
    setStep,
    setLoginStateToken,
    setAvailableMethods,
    setMfaMethod,
    setSessionToken,
  }: SubmitActionOptions,
  finishLogin: (session: string | null) => Promise<void>,
): LoginFormHandler {
  return async (event) => {
    event.preventDefault();
    await submitPasswordStep({
      loginStateToken,
      password,
      email,
      submitPassword: mutations.loginPasswordMutation.mutateAsync,
      setError,
      setStep,
      setLoginStateToken,
      setAvailableMethods,
      setMfaMethod,
      setSessionToken,
      navigateToVerifyEmail: (nextEmail, resendAvailableAt, accountName) => {
        void navigate({
          to: '/verify',
          state: (state) => ({
            ...state,
            accountName,
            email: nextEmail,
            resendAvailableAt,
          }),
        });
      },
      finishLogin,
    });
  };
}
