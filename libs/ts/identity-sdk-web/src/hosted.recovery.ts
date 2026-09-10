import { hostedRequest, text, type HostedTransport } from './hosted.transport';
import { credentialId, credentialLabel } from './hosted.webauthn.fields';
import { hostedCreationOptions } from './hosted.webauthn.options';
import { createWebAuthnCredential } from './webauthn.credentials';
import { serializeCredential } from './webauthn.codec';
import type { WebauthnCreateOptions } from './webauthn.types';

/** Keep in memory on Identity only. This proof never authorizes OAuth or step-up. */
export interface HostedMfaRecovery {
  csrfToken: string;
  expiresAt: string;
}

export interface HostedMfaRecovered {
  credentialId: string;
  mustReauthenticate: true;
}

function recoveryCode(value: unknown): string {
  const result = text(value, 48);
  if (!/^nvr1_[A-Za-z0-9_-]{43}$/.test(result)) throw new Error('Invalid Identity recovery code.');
  return result;
}

function recoveryProof(value: unknown): string {
  const result = text(value, 43);
  if (!/^[A-Za-z0-9_-]{43}$/.test(result)) throw new Error('Invalid Identity recovery proof.');
  return result;
}

function liveExpiry(value: unknown): string {
  const result = text(value, 64);
  if (!/^\d{4}-\d{2}-\d{2}T/.test(result) || !(Date.parse(result) > Date.now()))
    throw new Error('Invalid or expired Identity recovery session.');
  return result;
}

/** Sensitive one-time display: do not log or persist codes in browser storage. */
export async function generateHostedRecoveryCodes(
  config: HostedTransport,
  sessionCsrf: string,
): Promise<string[]> {
  const result = await hostedRequest(
    config,
    '/oauth/session/recovery/codes/generate',
    {},
    sessionCsrf,
  );
  if (!Array.isArray(result.codes) || result.codes.length !== 10)
    throw new Error('Invalid Identity recovery codes response.');
  const codes = result.codes.map(recoveryCode);
  if (new Set(codes).size !== 10) throw new Error('Duplicate Identity recovery codes.');
  return codes;
}

/** Success revokes ordinary sessions. Discard their proofs and OAuth state. */
export async function redeemHostedRecoveryCode(
  config: HostedTransport,
  sessionCsrf: string,
  code: string,
): Promise<HostedMfaRecovery> {
  const result = await hostedRequest(
    config,
    '/oauth/session/recovery/redeem',
    { code: recoveryCode(code) },
    sessionCsrf,
  );
  if (result.recovery !== true) throw new Error('Identity did not confirm recovery.');
  return { csrfToken: recoveryProof(result.csrf_token), expiresAt: liveExpiry(result.expires_at) };
}

/** Only a verified replacement completes recovery; a fresh login is still required. */
export async function completeHostedMfaRecovery(
  config: HostedTransport,
  recovery: HostedMfaRecovery,
  label: string,
  browserOptions?: WebauthnCreateOptions,
): Promise<HostedMfaRecovered> {
  const checkedLabel = credentialLabel(label);
  const csrf = recoveryProof(recovery.csrfToken);
  liveExpiry(recovery.expiresAt);
  const start = await hostedRequest(config, '/oauth/recovery/registration/options', {}, csrf);
  const ceremony = credentialId(start.ceremony_id);
  const credential = await createWebAuthnCredential(
    hostedCreationOptions(start.options),
    browserOptions,
  );
  liveExpiry(recovery.expiresAt);
  const result = await hostedRequest(
    config,
    '/oauth/recovery/registration/finish',
    {
      ceremony_id: ceremony,
      credential: serializeCredential(credential),
      label: checkedLabel,
    },
    csrf,
  );
  if (result.recovered !== true || result.must_reauthenticate !== true)
    throw new Error('Identity did not confirm recovery completion and reauthentication.');
  return { credentialId: credentialId(result.credential_id), mustReauthenticate: true };
}
