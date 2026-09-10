import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { authCredentials, authHeaders, handleMfaResponseError } from './mfa.transport';

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
