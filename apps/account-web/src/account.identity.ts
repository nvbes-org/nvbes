import { createHttpClient } from '@nvbes/http-client';
import { createAccountIdentityClient } from '@nvbes/identity-client';
import { verifiedFetch } from '@nvbes/web-runtime';
import {
  confirmTotp,
  generateRecoveryCodes,
  listMfaFactors,
  registerWebAuthnCredential,
  removeMfaFactor,
  setupTotp,
  stepUp,
  type StepUpCredentials,
} from '@nvbes/identity-sdk-web';
import { z } from 'zod';
import { getAccountAccessToken } from './account.oauth.access-token';

const SuccessSchema = z.object({ success: z.boolean() });

export function accountIdentityClient() {
  const token = requiredIdentityToken();
  return createAccountIdentityClient({
    http: createHttpClient({
      baseUrl: identityBaseUrl(),
      credentials: 'omit',
      headers: { Authorization: `Bearer ${token}` },
      fetchImpl: bearerOnlyFetch(token),
    }),
  });
}

export function listAccountMfaFactors(options?: { limit?: number; cursor?: string }) {
  return listMfaFactors(identityBaseUrl(), requiredIdentityToken(), options);
}

export function beginAccountTotp(label?: string) {
  return setupTotp(identityBaseUrl(), label, requiredIdentityToken());
}

export function confirmAccountTotp(factorId: string, code: string) {
  return confirmTotp(identityBaseUrl(), factorId, code, requiredIdentityToken());
}

export function registerAccountWebAuthn(kind: 'passkey' | 'security_key', label?: string) {
  return registerWebAuthnCredential(identityBaseUrl(), label, kind, requiredIdentityToken(), {
    timeoutMs: 60_000,
  });
}

export function removeAccountMfaFactor(factorId: string) {
  return removeMfaFactor(identityBaseUrl(), factorId, requiredIdentityToken());
}

export function generateAccountRecoveryCodes() {
  return generateRecoveryCodes(identityBaseUrl(), undefined, requiredIdentityToken());
}

export function verifyAccountStepUp(credentials: StepUpCredentials) {
  return stepUp(identityBaseUrl(), credentials, requiredIdentityToken());
}

export function changeAccountPassword(newPassword: string): Promise<void> {
  return identityHttp()
    .post('/auth/password/change', SuccessSchema, { new_password: newPassword })
    .then(() => undefined);
}

export function requestAccountPasswordReset(email: string): Promise<void> {
  return identityHttp(false)
    .post('/auth/password/forgot', SuccessSchema, { email })
    .then(() => undefined);
}

export function resetAccountPassword(token: string, newPassword: string): Promise<void> {
  return identityHttp(false)
    .post('/auth/password/reset', SuccessSchema, { token, new_password: newPassword })
    .then(() => undefined);
}

function identityHttp(authenticated = true) {
  const token = authenticated ? requiredIdentityToken() : null;
  return createHttpClient({
    baseUrl: identityBaseUrl(),
    credentials: 'omit',
    headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    fetchImpl: token ? bearerOnlyFetch(token) : verifiedFetch,
  });
}

function bearerOnlyFetch(token: string): typeof fetch {
  return async (input, init = {}) => {
    const headers = new Headers(init.headers);
    headers.set('Authorization', `Bearer ${token}`);
    headers.delete('Cookie');
    headers.delete('X-Auth-User');
    headers.delete('X-CSRF-Token');
    return verifiedFetch(input, { ...init, credentials: 'omit', headers });
  };
}

function requiredIdentityToken(): string {
  const token = getAccountAccessToken();
  if (!token) {
    throw new Error('La session Account a expiré. Reconnectez-vous.');
  }
  return token;
}

function identityBaseUrl(): string {
  const configured = import.meta.env.VITE_IDENTITY_SERVICE_BASE_URL?.trim();
  if (!configured) {
    throw new Error('VITE_IDENTITY_SERVICE_BASE_URL is required.');
  }
  return configured.replace(/\/+$/u, '');
}
