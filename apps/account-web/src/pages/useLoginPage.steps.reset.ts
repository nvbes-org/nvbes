import { preventAutoSignIn } from '@nvbes/identity-sdk-web';

import type { ResetMfaState } from './useLoginPage.steps.shared';

type ResetLoginFlowOptions = {
  setStep: (value: 'identifier') => void;
  setLoginStateToken: (value: string | null) => void;
  setSessionToken: (value: string | null) => void;
  resetMfaState: ResetMfaState;
  setError: (value: string | null) => void;
  setPassword: (value: string) => void;
};

export function resetLoginFlow({
  setStep,
  setLoginStateToken,
  setSessionToken,
  resetMfaState,
  setError,
  setPassword,
}: ResetLoginFlowOptions) {
  void preventAutoSignIn().catch(() => {});
  setStep('identifier');
  setLoginStateToken(null);
  setSessionToken(null);
  resetMfaState();
  setError(null);
  setPassword('');
}
