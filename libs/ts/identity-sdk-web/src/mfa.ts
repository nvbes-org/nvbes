import { createRequestHeaders } from '@nvbes/http-client';
import type {
  MfaFactorView,
  RecoveryCodesResult,
  TotpSetupResult,
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

export interface TotpSetupRequest {
  password?: string;
  label?: string;
}

export interface TotpConfirmRequest {
  factorId: string;
  code: string;
}

export interface WebauthnRegisterFinishRequest {
  factorId: string;
  credential: unknown;
}

export interface MfaEmailAddress {
  id: string;
  email: string;
  is_primary: boolean;
  verified: boolean;
  verified_at: string | null;
  created_at: string;
}

function readCsrfToken(): string | undefined {
  if (typeof document === 'undefined') {
    return undefined;
  }
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
  return match?.[1] || undefined;
}

function authHeaders(token?: string, method = 'GET'): Headers {
  const headers = new Headers();
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  const upperMethod = method.toUpperCase();
  if (['POST', 'PUT', 'PATCH', 'DELETE'].includes(upperMethod)) {
    const csrfToken = readCsrfToken();
    if (csrfToken) {
      headers.set('X-CSRF-Token', csrfToken);
    }
  }
  return createRequestHeaders(method, headers);
}

function jsonAuthHeaders(token?: string, method = 'POST'): Headers {
  const headers = authHeaders(token, method);
  headers.set('Content-Type', 'application/json');
  return headers;
}

const MFA_ERROR_MESSAGES: Record<string, string> = {
  invalid_credentials: 'Les informations saisies sont incorrectes.',
  invalid_totp_code: "Le code d'authentification est incorrect.",
  invalid_recovery_code: 'Le code de récupération est incorrect.',
  invalid_email_mfa_code: 'Le code reçu par email est incorrect ou a expiré.',
  email_mfa_not_configured: "La vérification par email n'est pas disponible pour ce compte.",
  email_mfa_requires_verified_email: 'Ajoutez un email vérifié avant d’utiliser cette méthode.',
  email_mfa_cannot_be_removed: 'La vérification par email est requise et ne peut pas être retirée.',
  step_up_required: 'Confirmez votre identité pour continuer.',
  step_up_expired: 'La vérification a expiré. Recommencez.',
  webauthn_challenge_missing: 'La vérification par clé de sécurité doit être relancée.',
  webauthn_challenge_expired: 'La demande de clé de sécurité a expiré. Recommencez.',
  webauthn_verification_failed: 'La clé de sécurité n’a pas pu être vérifiée.',
  mfa_factor_not_found: 'Cette méthode de vérification est introuvable.',
  challenge_locked: 'Trop de tentatives. Réessayez dans quelques minutes.',
  validation_failed: 'Vérifiez les informations saisies.',
  unauthorized: 'Votre session a expiré. Reconnectez-vous.',
  forbidden: 'Vous ne pouvez pas effectuer cette action.',
};

function mfaErrorMessage(code: string, fallback?: string) {
  return MFA_ERROR_MESSAGES[code] ?? fallback ?? 'La vérification a échoué. Veuillez réessayer.';
}

async function handleResponse(response: Response, _context: string): Promise<never> {
  const body = await response.json().catch(() => ({}));
  const code = body?.error?.code ?? response.statusText;
  throw new MfaError(code, mfaErrorMessage(code), response.status);
}

export class MfaError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    public readonly status: number,
  ) {
    super(message);
    this.name = 'MfaError';
  }
}

/**
 * Liste les facteurs MFA de l'utilisateur connecté.
 * Quand `token` est absent, l'auth se fait via cookie de session (`credentials: 'include'`).
 */
export async function listMfaFactors(
  baseUrl: string,
  token?: string,
): Promise<{ factors: MfaFactorView[]; mfa_enabled: boolean }> {
  const response = await fetch(`${baseUrl}/auth/mfa/factors`, {
    headers: authHeaders(token),
    credentials: 'include',
  });

  if (!response.ok) {
    return handleResponse(response, 'list factors');
  }

  return response.json();
}

/**
 * Configure TOTP (génère secret et URI de provisioning pour QR code).
 * Le backend requiert un step-up préalable (cookie ou token Bearer).
 */
export async function setupTotp(
  baseUrl: string,
  label?: string,
  token?: string,
): Promise<TotpSetupResult> {
  const response = await fetch(`${baseUrl}/auth/mfa/totp/setup`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({ label }),
  });

  if (!response.ok) {
    return handleResponse(response, 'TOTP setup');
  }

  return response.json();
}

/**
 * Confirme le code TOTP (active le facteur).
 */
export async function confirmTotp(
  baseUrl: string,
  factorId: string,
  code: string,
  token?: string,
): Promise<{ factor: MfaFactorView; mfa_enabled: boolean }> {
  const response = await fetch(`${baseUrl}/auth/mfa/totp/confirm`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({ factor_id: factorId, code }),
  });

  if (!response.ok) {
    return handleResponse(response, 'TOTP confirm');
  }

  return response.json();
}

export async function listEmailMfaEligible(
  baseUrl: string,
  token?: string,
): Promise<{ emails: MfaEmailAddress[]; primary_min_age_hours: number }> {
  const response = await fetch(`${baseUrl}/auth/mfa/email/eligible`, {
    headers: authHeaders(token),
    credentials: 'include',
  });

  if (!response.ok) {
    return handleResponse(response, 'email eligible');
  }

  return response.json();
}

export async function setupEmailMfa(
  baseUrl: string,
  emailId: string,
  token?: string,
): Promise<{ factor: MfaFactorView; mfa_enabled: boolean }> {
  const response = await fetch(`${baseUrl}/auth/mfa/email/setup`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({ email_id: emailId }),
  });

  if (!response.ok) {
    return handleResponse(response, 'email setup');
  }

  return response.json();
}

/**
 * Démarre l'enregistrement WebAuthn (récupère le défi du serveur).
 */
export async function startWebAuthnRegistration(
  baseUrl: string,
  label?: string,
  kind: WebauthnRegistrationKind = 'passkey',
  token?: string,
): Promise<{ factorId: string; options: PublicKeyCredentialCreationOptions }> {
  const response = await fetch(`${baseUrl}/auth/mfa/webauthn/register/start`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({ label, kind }),
  });

  if (!response.ok) {
    return handleResponse(response, 'WebAuthn registration start');
  }

  const data: WebauthnRegisterStartResult = await response.json();

  return {
    factorId: data.factor_id,
    options: parseCreationOptions(data.options),
  };
}

/**
 * Termine l'enregistrement WebAuthn (envoie la réponse du navigateur au serveur).
 */
export async function finishWebAuthnRegistration(
  baseUrl: string,
  factorId: string,
  credential: PublicKeyCredential,
  token?: string,
): Promise<void> {
  const serialized = serializeCredential(credential);

  const response = await fetch(`${baseUrl}/auth/mfa/webauthn/register/finish`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({ factor_id: factorId, reg: serialized }),
  });

  if (!response.ok) {
    return handleResponse(response, 'WebAuthn registration finish');
  }
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

/**
 * Démarre l'authentification WebAuthn (pour step-up ou login MFA).
 */
export async function startWebAuthnAuthentication(
  baseUrl: string,
  token?: string,
): Promise<{
  challengeId: string;
  options: PublicKeyCredentialRequestOptions;
}> {
  const response = await fetch(`${baseUrl}/auth/mfa/webauthn/start`, {
    method: 'POST',
    headers: authHeaders(token, 'POST'),
    credentials: 'include',
  });

  if (!response.ok) {
    return handleResponse(response, 'WebAuthn auth start');
  }

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
): Promise<void> {
  const { challengeId, options } = await startWebAuthnAuthentication(baseUrl, token);
  const credential = await getWebAuthnCredential(options, getOptions);
  const serialized = serializeCredential(credential);
  await stepUp(baseUrl, { webauthnResponse: serialized, webauthnChallengeId: challengeId }, token);
}

/**
 * Génère de nouveaux codes de récupération.
 * Le backend requiert un step-up préalable.
 */
export async function generateRecoveryCodes(
  baseUrl: string,
  password: string,
  token?: string,
): Promise<RecoveryCodesResult> {
  const response = await fetch(`${baseUrl}/auth/mfa/recovery-codes`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({ password }),
  });

  if (!response.ok) {
    return handleResponse(response, 'recovery codes');
  }

  return response.json();
}

/**
 * Supprime un facteur MFA.
 * Le backend requiert un step-up préalable.
 */
export async function removeMfaFactor(
  baseUrl: string,
  factorId: string,
  token?: string,
): Promise<void> {
  const response = await fetch(`${baseUrl}/auth/mfa/factors/${factorId}`, {
    method: 'DELETE',
    headers: authHeaders(token, 'DELETE'),
    credentials: 'include',
  });

  if (!response.ok) {
    return handleResponse(response, 'remove factor');
  }
}

/**
 * Step-up : re-vérifie l'identité avant une opération sensible.
 * Au moins un credential doit être fourni.
 */
export async function stepUp(
  baseUrl: string,
  credentials: {
    password?: string;
    totpCode?: string;
    webauthnResponse?: unknown;
    webauthnChallengeId?: string;
    recoveryCode?: string;
  },
  token?: string,
): Promise<{ success: boolean; valid_until: string }> {
  const response = await fetch(`${baseUrl}/auth/step-up`, {
    method: 'POST',
    headers: jsonAuthHeaders(token, 'POST'),
    credentials: 'include',
    body: JSON.stringify({
      password: credentials.password ?? null,
      totp_code: credentials.totpCode ?? null,
      webauthn_response: credentials.webauthnResponse ?? null,
      webauthn_challenge_id: credentials.webauthnChallengeId ?? null,
      recovery_code: credentials.recoveryCode ?? null,
    }),
  });

  if (!response.ok) {
    return handleResponse(response, 'step-up');
  }

  return response.json();
}
