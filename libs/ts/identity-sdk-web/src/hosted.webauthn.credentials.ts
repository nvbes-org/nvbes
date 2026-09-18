import {
  hostedJsonRequest,
  hostedRequest,
  record,
  text,
  type HostedTransport,
} from './hosted.transport';
import { credentialId, credentialLabel } from './hosted.webauthn.fields';

export interface HostedPasskey {
  id: string;
  label: string;
  createdAt: string;
  lastUsedAt: string | null;
}

function timestamp(value: unknown): string {
  const date = text(value, 64);
  if (!Number.isFinite(Date.parse(date))) throw new Error('Invalid Identity credential date.');
  return date;
}

export async function listHostedPasskeys(
  config: HostedTransport,
  sessionCsrf: string,
): Promise<HostedPasskey[]> {
  const result = await hostedJsonRequest(
    config,
    '/oauth/session/webauthn/credentials/list',
    {},
    sessionCsrf,
  );
  if (!Array.isArray(result) || result.length > 10)
    throw new Error('Invalid Identity credential list.');
  const ids = new Set<string>();
  return result.map((entry: unknown) => {
    const item = record(entry);
    const id = credentialId(item.id);
    if (ids.has(id)) throw new Error('Duplicate Identity credential.');
    ids.add(id);
    return {
      id,
      label: credentialLabel(item.label),
      createdAt: timestamp(item.created_at),
      lastUsedAt: item.last_used_at === null ? null : timestamp(item.last_used_at),
    };
  });
}

export async function renameHostedPasskey(
  config: HostedTransport,
  sessionCsrf: string,
  id: string,
  label: string,
): Promise<void> {
  const result = await hostedRequest(
    config,
    '/oauth/session/webauthn/credentials/rename',
    {
      credential_id: credentialId(id),
      label: credentialLabel(label),
    },
    sessionCsrf,
  );
  if (result.renamed !== true) throw new Error('Identity did not confirm credential rename.');
}

/** Revocation also ends sessions authenticated with this key, possibly the caller's. */
export async function revokeHostedPasskey(
  config: HostedTransport,
  sessionCsrf: string,
  id: string,
): Promise<void> {
  const result = await hostedRequest(
    config,
    '/oauth/session/webauthn/credentials/revoke',
    {
      credential_id: credentialId(id),
    },
    sessionCsrf,
  );
  if (result.revoked !== true) throw new Error('Identity did not confirm credential revocation.');
}
