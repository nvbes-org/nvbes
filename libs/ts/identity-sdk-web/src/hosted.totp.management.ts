import {
  hostedJsonRequest,
  hostedRequest,
  record,
  text,
  type HostedTransport,
} from './hosted.transport';

export interface HostedTotpFactor {
  id: string;
  createdAt: string;
}

function factorId(value: unknown): string {
  const id = text(value, 36);
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(id))
    throw new Error('Invalid TOTP factor identifier.');
  return id;
}

export async function listHostedTotpFactors(
  config: HostedTransport,
  sessionCsrf: string,
): Promise<HostedTotpFactor[]> {
  const result = await hostedJsonRequest(
    config,
    '/oauth/session/totp/factors/list',
    {},
    sessionCsrf,
  );
  if (!Array.isArray(result) || result.length > 1) throw new Error('Invalid TOTP factor list.');
  return result.map((value: unknown) => {
    const row = record(value);
    const createdAt = text(row.created_at, 64);
    if (!Number.isFinite(Date.parse(createdAt))) throw new Error('Invalid TOTP factor date.');
    return { id: factorId(row.id), createdAt };
  });
}

/** Success ends all Identity sessions; the caller must start a new authorization. */
export async function revokeHostedTotpFactor(
  config: HostedTransport,
  sessionCsrf: string,
  id: string,
): Promise<void> {
  const result = await hostedRequest(
    config,
    '/oauth/session/totp/factors/revoke',
    { factor_id: factorId(id) },
    sessionCsrf,
  );
  if (result.revoked !== true || result.sessions_revoked !== true)
    throw new Error('Identity did not confirm TOTP and session revocation.');
}
