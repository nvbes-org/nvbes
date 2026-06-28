import type { WebauthnRequestOptionsJSON } from '@nvbes/identity-sdk-core/src/types';
import {
  getConditionalWebAuthnCredential,
  parseRequestOptions,
  serializeCredential,
} from '@nvbes/identity-sdk-web';

import {
  finishDiscoverableLoginWebAuthn,
  startDiscoverableLoginWebAuthn,
} from '../identity.auth.api';

type ConditionalWebAuthnLoginOptions = {
  finishLogin: (session: string | null) => Promise<void>;
  setSessionToken: (value: string | null) => void;
  setError: (value: string | null) => void;
};

export async function completeConditionalWebAuthnLogin({
  finishLogin,
  setSessionToken,
  setError,
}: ConditionalWebAuthnLoginOptions) {
  const start = await startDiscoverableLoginWebAuthn();
  const publicKey = parseRequestOptions(
    start.options as WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
  );
  const credential = await getConditionalWebAuthnCredential(publicKey);
  if (!credential) {
    return;
  }

  const result = await finishDiscoverableLoginWebAuthn(
    start.challenge_id,
    serializeCredential(credential),
  ).catch((error: unknown) => {
    setError(error instanceof Error ? error.message : 'Connexion biométrique impossible.');
    return null;
  });
  if (!result) {
    return;
  }
  if (result.session_token) {
    setSessionToken(result.session_token);
  }
  await finishLogin(result.session_token ?? null);
  setError(null);
}
