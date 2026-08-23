import { verifiedFetch } from '@nvbes/web-runtime';
import type { AdminCredentials } from './backoffice-service.types';

const jsonHeaders = { 'content-type': 'application/json' };

export function authHeaders(credentials: AdminCredentials): Record<string, string> {
  const headers: Record<string, string> = {
    'x-nvbes-internal-token': credentials.internalToken,
    'x-nvbes-actor-principal-id': credentials.actorPrincipalId,
    'x-nvbes-backoffice-role': credentials.backofficeRole,
  };
  if (credentials.secondApproverPrincipalId.trim()) {
    headers['x-nvbes-second-approver-principal-id'] = credentials.secondApproverPrincipalId;
    headers['x-nvbes-second-approver-role'] = credentials.secondApproverRole;
  }
  return headers;
}

export function mutationHeaders(credentials: AdminCredentials): Record<string, string> {
  return {
    ...authHeaders(credentials),
    'Idempotency-Key': crypto.randomUUID(),
  };
}

export async function parseJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Back-office request failed with HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

export async function postJson<TBody, TResult>(
  credentials: AdminCredentials,
  path: string,
  body: TBody,
): Promise<TResult> {
  const response = await verifiedFetch(path, {
    method: 'POST',
    headers: { ...jsonHeaders, ...mutationHeaders(credentials) },
    body: JSON.stringify(body),
  });
  return parseJson<TResult>(response);
}
