import type {
  MfaFactorView,
  RecoveryCodesResult,
  TotpSetupResult,
} from '@nvbes/identity-sdk-core/src/types';
import {
  authCredentials,
  authHeaders,
  handleMfaResponseError,
  jsonAuthHeaders,
} from './mfa.transport';

export interface TotpSetupRequest {
  password?: string;
  label?: string;
}

export interface TotpConfirmRequest {
  factorId: string;
  code: string;
}

export async function listMfaFactors(
  baseUrl: string,
  token?: string,
  options: { limit?: number; cursor?: string } = {},
): Promise<{
  factors: MfaFactorView[];
  mfa_enabled: boolean;
  next_cursor: string | null;
  has_more: boolean;
}> {
  const params = new URLSearchParams();
  if (options.limit !== undefined) params.set('limit', String(options.limit));
  if (options.cursor) params.set('cursor', options.cursor);
  const query = params.toString();
  const response = await fetch(`${baseUrl}/auth/mfa/factors${query ? `?${query}` : ''}`, {
    headers: authHeaders(token),
    credentials: authCredentials(token),
  });
  if (!response.ok) return handleMfaResponseError(response);
  return response.json();
}

export async function setupTotp(
  baseUrl: string,
  label?: string,
  token?: string,
): Promise<TotpSetupResult> {
  const response = await fetch(`${baseUrl}/auth/mfa/totp/setup`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: authCredentials(token),
    body: JSON.stringify({ label }),
  });
  if (!response.ok) return handleMfaResponseError(response);
  return response.json();
}

export async function confirmTotp(
  baseUrl: string,
  factorId: string,
  code: string,
  token?: string,
): Promise<{ factor: MfaFactorView; mfa_enabled: boolean }> {
  const response = await fetch(`${baseUrl}/auth/mfa/totp/confirm`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: authCredentials(token),
    body: JSON.stringify({ factor_id: factorId, code }),
  });
  if (!response.ok) return handleMfaResponseError(response);
  return response.json();
}

export async function generateRecoveryCodes(
  baseUrl: string,
  password?: string,
  token?: string,
): Promise<RecoveryCodesResult> {
  const response = await fetch(`${baseUrl}/auth/mfa/recovery-codes`, {
    method: 'POST',
    headers: jsonAuthHeaders(token),
    credentials: authCredentials(token),
    body: JSON.stringify({ password: password ?? '' }),
  });
  if (!response.ok) return handleMfaResponseError(response);
  return response.json();
}

export async function removeMfaFactor(
  baseUrl: string,
  factorId: string,
  token?: string,
): Promise<void> {
  const response = await fetch(`${baseUrl}/auth/mfa/factors/${factorId}`, {
    method: 'DELETE',
    headers: authHeaders(token, 'DELETE'),
    credentials: authCredentials(token),
  });
  if (!response.ok) return handleMfaResponseError(response);
}
