import type {
  WebauthnAuthStartResult,
  WebauthnRegisterStartResult,
} from '@nvbes/identity-sdk-core/src/types';
import {
  createWebAuthnCredential,
  getWebAuthnCredential,
  parseCreationOptions,
  parseRequestOptions,
  serializeCredential,
  type WebauthnCreateOptions,
  type WebauthnRegistrationKind,
} from './webauthn';
import { stepUp, type StepUpPurpose } from './mfa.step-up';
import {
  authHeaders,
  handleMfaResponseError,
  jsonAuthHeaders,
  MfaError,
  mfaErrorMessage,
} from './mfa.transport';

export interface WebauthnRegisterFinishRequest {
  factorId: string;
  credential: unknown;
}

export async function startWebAuthnRegistration(
  baseUrl: string,
  label?: string,
  kind: WebauthnRegistrationKind = 'passkey',
  token?: string,
): Promise<{ factorId: string; options: PublicKeyCredentialCreationOptions }> {
  const response = await fetch(`${baseUrl}/auth/mfa/webauthn/register/start`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: 'include',
    body: JSON.stringify({ label, kind }),
  });
  if (!response.ok) return handleMfaResponseError(response);

  const data: WebauthnRegisterStartResult = await response.json();
  return {
    factorId: data.factor_id,
    options: parseCreationOptions(data.options),
  };
}

export async function finishWebAuthnRegistration(
  baseUrl: string,
  factorId: string,
  credential: PublicKeyCredential,
  token?: string,
): Promise<void> {
  const response = await fetch(`${baseUrl}/auth/mfa/webauthn/register/finish`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: 'include',
    body: JSON.stringify({ factor_id: factorId, reg: serializeCredential(credential) }),
  });
  if (!response.ok) return handleMfaResponseError(response);
}

export async function registerWebAuthnCredential(
  baseUrl: string,
  label?: string,
  kind: WebauthnRegistrationKind = 'passkey',
  token?: string,
  createOptions?: WebauthnCreateOptions,
): Promise<void> {
  const { factorId, options } = await startWebAuthnRegistration(baseUrl, label, kind, token);
  const credential = await createWebAuthnCredential(options, createOptions);
  await finishWebAuthnRegistration(baseUrl, factorId, credential, token);
}

export async function startWebAuthnAuthentication(
  baseUrl: string,
  token?: string,
): Promise<{ challengeId: string; options: PublicKeyCredentialRequestOptions }> {
  const response = await fetch(`${baseUrl}/auth/mfa/webauthn/start`, {
    method: 'POST',
    headers: authHeaders(token, 'POST'),
    credentials: 'include',
  });
  if (!response.ok) return handleMfaResponseError(response);

  const data: WebauthnAuthStartResult = await response.json();
  if (!data.challenge_id) {
    throw new MfaError(
      'webauthn_challenge_missing',
      mfaErrorMessage('webauthn_challenge_missing'),
      response.status,
    );
  }
  return {
    challengeId: data.challenge_id,
    options: parseRequestOptions(data.options),
  };
}

export async function completeWebAuthnStepUp(
  baseUrl: string,
  token?: string,
  getOptions?: WebauthnCreateOptions,
  purpose?: StepUpPurpose,
): Promise<void> {
  const { challengeId, options } = await startWebAuthnAuthentication(baseUrl, token);
  const credential = await getWebAuthnCredential(options, getOptions);
  await stepUp(
    baseUrl,
    {
      purpose,
      webauthnResponse: serializeCredential(credential),
      webauthnChallengeId: challengeId,
    },
    token,
  );
}
