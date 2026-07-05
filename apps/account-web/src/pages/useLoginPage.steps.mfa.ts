import type { WebauthnRequestOptionsJSON } from '@nvbes/identity-sdk-core/src/types';
import {
  getWebAuthnCredential,
  parseRequestOptions as parseLoginWebAuthnRequestOptions,
  serializeCredential,
  WebauthnBrowserError,
} from '@nvbes/identity-sdk-web';

import type { MfaMethod } from './LoginPage.mfa';
import { isInvalidSignatureError } from './LoginPage.errors';
import { WEBAUTHN_TIMEOUT_MS } from './useLoginPage.shared';
import type {
  MfaMutateAsync,
  MfaStepResult,
  WebauthnStartMutateAsync,
} from './useLoginPage.steps.shared';

type SubmitMfaStepOptions = {
  loginStateToken: string | null;
  sessionToken: string | null;
  mfaMethod: MfaMethod | null;
  totpCode: string;
  emailCode: string;
  recoveryCode: string;
  submitMfa: MfaMutateAsync;
  startWebauthn: WebauthnStartMutateAsync;
  setError: (value: string | null) => void;
  setStep: (value: 'identifier') => void;
  setLoginStateToken: (value: string | null) => void;
  setSessionToken: (value: string | null) => void;
  finishLogin: (sessionToken: string | null) => Promise<void>;
};

export async function submitMfaStep({
  loginStateToken,
  sessionToken,
  mfaMethod,
  totpCode,
  emailCode,
  recoveryCode,
  submitMfa,
  startWebauthn,
  setError,
  setStep,
  setLoginStateToken,
  setSessionToken,
  finishLogin,
}: SubmitMfaStepOptions) {
  if (!loginStateToken) {
    setError('Session de connexion expirée. Recommencez.');
    setStep('identifier');
    return;
  }

  if (!mfaMethod) {
    return;
  }

  setError(null);
  try {
    let result: MfaStepResult | undefined;

    if (mfaMethod === 'totp') {
      result = await submitMfa({
        stateToken: loginStateToken,
        totpCode,
      });
    } else if (mfaMethod === 'email') {
      result = await submitMfa({
        stateToken: loginStateToken,
        emailCode,
      });
    } else if (mfaMethod === 'recovery') {
      result = await submitMfa({
        stateToken: loginStateToken,
        recoveryCode,
      });
    } else {
      const start = await startWebauthn(loginStateToken);
      const options = parseLoginWebAuthnRequestOptions(
        start.options as WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
      );
      const credential = await getWebAuthnCredential(options, {
        timeoutMs: WEBAUTHN_TIMEOUT_MS,
      });
      result = await submitMfa({
        stateToken: loginStateToken,
        webauthnChallengeId: start.challenge_id,
        webauthnResponse: serializeCredential(credential),
      });
    }

    if (result?.session_token) {
      setSessionToken(result.session_token);
    }
    await finishLogin(result?.session_token ?? sessionToken);
  } catch (err) {
    if (isInvalidSignatureError(err)) {
      setLoginStateToken(null);
      setSessionToken(null);
      setStep('identifier');
      return;
    }

    if (err instanceof WebauthnBrowserError && err.code === 'webauthn_timeout') {
      setError('WebAuthn request timed out. Please try again.');
      return;
    }

    setError(err instanceof Error ? err.message : 'MFA verification failed');
  }
}
