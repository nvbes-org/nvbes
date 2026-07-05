import { resolvePowChallenge } from '../identity.auth.pow';
import { accountServiceBaseUrl } from '../identity.http';
import { submitIdentifierStep } from './useLoginPage.steps';
import type { LoginFormHandler, SubmitActionOptions } from './useLoginPage.actions.submit.shared';

export function buildIdentifierSubmitAction({
  email,
  decoyRef,
  mutations,
  resetMfaState,
  setLoginStateToken,
  setSessionToken,
  setAvailableMethods,
  setMfaMethod,
  setStep,
  setError,
  setIdentifierSubmitting,
}: SubmitActionOptions): LoginFormHandler {
  return async (event) => {
    event.preventDefault();
    setIdentifierSubmitting(true);
    try {
      await submitIdentifierStep({
        email,
        decoyLinkClicked: decoyRef.current?.wasClicked() ?? false,
        powChallenge: await resolvePowChallenge(accountServiceBaseUrl),
        submitIdentifier: mutations.loginIdentifierMutation.mutateAsync,
        resetMfaState,
        setLoginStateToken,
        setSessionToken,
        setAvailableMethods,
        setMfaMethod,
        setStep,
        setError,
      });
    } finally {
      setIdentifierSubmitting(false);
    }
  };
}
