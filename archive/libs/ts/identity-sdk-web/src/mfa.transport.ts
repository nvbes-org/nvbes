import { createRequestHeaders } from '@nvbes/http-client';
import { readCurrentAuthuser, readScopedCsrfToken } from './csrf';

const MFA_ERROR_MESSAGES: Record<string, string> = {
  invalid_credentials: 'Les informations saisies sont incorrectes.',
  invalid_totp_code: "Le code d'authentification est incorrect.",
  invalid_recovery_code: 'Le code de récupération est incorrect.',
  step_up_required: 'Confirmez votre identité pour continuer.',
  phishing_resistant_step_up_required:
    'Confirmez cette action avec une passkey ou une clé de sécurité.',
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

export function authHeaders(token?: string, method = 'GET'): Headers {
  const headers = new Headers();
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
    return createRequestHeaders(method, headers);
  }

  const authuser = readCurrentAuthuser();
  if (authuser) headers.set('X-Auth-User', authuser);

  const upperMethod = method.toUpperCase();
  if (['POST', 'PUT', 'PATCH', 'DELETE'].includes(upperMethod)) {
    const csrfToken = readScopedCsrfToken(authuser);
    if (csrfToken) headers.set('X-CSRF-Token', csrfToken);
  }
  return createRequestHeaders(method, headers);
}

export function jsonAuthHeaders(token?: string, method = 'POST'): Headers {
  const headers = authHeaders(token, method);
  headers.set('Content-Type', 'application/json');
  return headers;
}

export function authCredentials(token?: string): RequestCredentials {
  return token ? 'omit' : 'include';
}

export async function handleMfaResponseError(response: Response): Promise<never> {
  const body: unknown = await response.json().catch(() => undefined);
  const code = responseErrorCode(body) ?? response.statusText;
  throw new MfaError(code, mfaErrorMessage(code), response.status);
}

export function mfaErrorMessage(code: string, fallback?: string): string {
  return MFA_ERROR_MESSAGES[code] ?? fallback ?? 'La vérification a échoué. Veuillez réessayer.';
}

function responseErrorCode(body: unknown): string | undefined {
  if (typeof body !== 'object' || body === null || !('error' in body)) return undefined;
  const error = body.error;
  if (typeof error !== 'object' || error === null || !('code' in error)) return undefined;
  return typeof error.code === 'string' ? error.code : undefined;
}
