import { accountTransport } from './account.transport';

export interface AccountConfig {
  identityOrigin: string;
  accountApiOrigin: string;
  clientId: string;
}

export function parseAccountConfig(value: unknown): AccountConfig {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error('Invalid Account configuration');
  const data = value as Record<string, unknown>;
  if (typeof data.clientId !== 'string' || !/^[\x21-\x7e]{1,200}$/u.test(data.clientId))
    throw new Error('Invalid OAuth client');
  return {
    identityOrigin: origin(data.identityOrigin),
    accountApiOrigin: origin(data.accountApiOrigin),
    clientId: data.clientId,
  };
}

function origin(value: unknown): string {
  if (typeof value !== 'string' || value.trim() !== value) throw new Error('Invalid origin');
  const url = new URL(value);
  const loopback =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if (
    (url.protocol !== 'https:' && !loopback) ||
    url.username ||
    url.password ||
    url.pathname !== '/' ||
    url.search ||
    url.hash
  )
    throw new Error('Invalid origin');
  return url.origin;
}

export async function loadAccountConfig(signal: AbortSignal): Promise<AccountConfig> {
  const response = await accountTransport([location.origin], signal)('/account-config.json', {
    signal,
    cache: 'no-store',
    credentials: 'omit',
    redirect: 'error',
    headers: { Accept: 'application/json' },
  });
  if (!response.ok) throw new Error('Account configuration unavailable');
  return parseAccountConfig(await response.json());
}
