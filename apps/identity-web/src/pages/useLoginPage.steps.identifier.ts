import { clientErrorMessage } from '@nvbes/web-runtime';

import type { MfaMethod } from './LoginPage.mfa';
import type { PowChallengeProof } from '../identity.auth.pow';
import { isInvalidSignatureError } from './LoginPage.errors';
import { normalizeMfaMethods } from './LoginPage.mfa';
import { preferredMfaMethod, requestsMfa } from './useLoginPage.shared';
import type { IdentifierMutateAsync, ResetMfaState } from './useLoginPage.steps.shared';

type SubmitIdentifierStepOptions = {
  email: string;
  decoyLinkClicked: boolean;
  powChallenge: PowChallengeProof;
  submitIdentifier: IdentifierMutateAsync;
  resetMfaState: ResetMfaState;
  setLoginStateToken: (value: string | null) => void;
  setSessionToken: (value: string | null) => void;
  setAvailableMethods: (value: MfaMethod[]) => void;
  setMfaMethod: (value: MfaMethod | null) => void;
  setStep: (value: 'identifier' | 'password' | 'webauthn' | 'mfa') => void;
  setError: (value: string | null) => void;
};

export async function submitIdentifierStep({
  email,
  decoyLinkClicked,
  powChallenge,
  submitIdentifier,
  resetMfaState,
  setLoginStateToken,
  setSessionToken,
  setAvailableMethods,
  setMfaMethod,
  setStep,
  setError,
}: SubmitIdentifierStepOptions) {
  setError(null);

  try {
    const result = await submitIdentifier({
      email,
      decoy_link_clicked: decoyLinkClicked,
      ...powChallenge,
    });

    setLoginStateToken(result.state_token);
    setSessionToken(null);
    resetMfaState();

    if (!requestsMfa(result.next_step, result.available_methods)) {
      setStep('password');
      return;
    }

    const availableMethods = normalizeMfaMethods(result.available_methods);
    const preferredMethod = preferredMfaMethod(result.available_methods);
    setAvailableMethods(availableMethods);
    setMfaMethod(preferredMethod);
    setStep(preferredMethod === 'webauthn' ? 'webauthn' : 'mfa');
  } catch (err) {
    if (isInvalidSignatureError(err)) {
      setLoginStateToken(null);
      setSessionToken(null);
      resetMfaState();
      setStep('identifier');
      return;
    }

    setError(clientErrorMessage(err, 'Login failed'));
  }
}
