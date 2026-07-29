import type { WebauthnRequestOptionsJSON } from '@nvbes/identity-sdk-core/src/types';
import { clientErrorMessage } from '@nvbes/web-runtime';
import {
  getConditionalWebAuthnCredential,
  parseRequestOptions,
  serializeCredential,
} from '@nvbes/identity-sdk-web';

import {
  finishDiscoverableLoginWebAuthn,
  startDiscoverableLoginWebAuthn,
} from '../identity.auth.api';
import type { LoginPasswordResult } from '../identity.auth.api';

type ConditionalWebAuthnLoginOptions = {
  finishLogin: (session: string | null) => Promise<void>;
  setSessionToken: (value: string | null) => void;
  setEmail: (value: string) => void;
  setLoginStateToken: (value: string | null) => void;
  setStep: (value: 'password') => void;
  setError: (value: string | null) => void;
  signal?: AbortSignal;
};

type ConditionalWebAuthnResultOptions = Omit<ConditionalWebAuthnLoginOptions, 'signal'>;

export async function applyConditionalWebAuthnResult(
  result: LoginPasswordResult,
  {
    finishLogin,
    setSessionToken,
    setEmail,
    setLoginStateToken,
    setStep,
    setError,
  }: ConditionalWebAuthnResultOptions,
) {
  if (result.next_step === 'pwd') {
    if (!result.state_token || !result.email) {
      setError('La session de connexion est invalide. Veuillez réessayer.');
      return;
    }
    setEmail(result.email);
    setLoginStateToken(result.state_token);
    setStep('password');
    setError(null);
    return;
  }

  if (result.session_token) {
    setSessionToken(result.session_token);
  }
  await finishLogin(result.session_token ?? null);
  setError(null);
}

export async function completeConditionalWebAuthnLogin({
  finishLogin,
  setSessionToken,
  setEmail,
  setLoginStateToken,
  setStep,
  setError,
  signal,
}: ConditionalWebAuthnLoginOptions) {
  const start = await startDiscoverableLoginWebAuthn();
  if (signal?.aborted) {
    return;
  }
  const publicKey = parseRequestOptions(
    start.options as WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
  );
  const credential = await getConditionalWebAuthnCredential(publicKey, {
    signal,
  });
  if (!credential || signal?.aborted) {
    return;
  }

  const result = await finishDiscoverableLoginWebAuthn(
    start.challenge_id,
    serializeCredential(credential),
  ).catch((error: unknown) => {
    if (signal?.aborted) {
      return null;
    }
    setError(clientErrorMessage(error, 'Connexion biométrique impossible.'));
    return null;
  });
  if (!result || signal?.aborted) {
    return;
  }
  await applyConditionalWebAuthnResult(result, {
    finishLogin,
    setSessionToken,
    setEmail,
    setLoginStateToken,
    setStep,
    setError,
  });
}
