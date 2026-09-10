import { generateCodeChallenge, generateCodeVerifier } from './pkce';
import { createDpopProof, generateBrowserDpopKeyPair, type DpopMainKeyPair } from './dpop';
import { IndexedDbDpopTransactionStore, type DpopTransactionStore } from './dpop.transaction-store';
import type { OAuthTransaction, WebStorage } from './storage';

export interface AuthorizationRequestConfig {
  baseUrl: string;
  clientId: string;
  redirectUri: string;
  resource: string;
  storage: WebStorage;
  fetchImpl?: typeof fetch;
  now?: () => number;
  dpop?: boolean;
  dpopStore?: DpopTransactionStore;
}

export interface AuthorizationRequestInput {
  scope?: string;
  resource?: string;
  state?: string;
  nonce?: string;
  returnTo?: string;
}

export interface AuthorizationRequest {
  authorizationUrl: string;
  state: string;
}

interface ParResponse {
  requestUri: string;
  expiresIn: number;
}

export async function createAuthorizationRequest(
  config: AuthorizationRequestConfig,
  input: AuthorizationRequestInput = {},
): Promise<AuthorizationRequest> {
  const state = transactionValue(input.state ?? generateRandomValue());
  const scope = input.scope?.trim() || 'openid';
  if (!scope.split(' ').includes('openid')) {
    throw new Error('Identity authorization requires the openid scope.');
  }
  const nonce = transactionValue(input.nonce ?? generateRandomValue());
  const resource = requiredValue(input.resource ?? config.resource, 'OAuth resource URL');
  const resourceUrl = new URL(resource);
  if (resourceUrl.hash || resourceUrl.username || resourceUrl.password) {
    throw new Error('OAuth resource URL cannot contain credentials or a fragment.');
  }
  normalizedBaseUrl(config.baseUrl);
  const codeVerifier = generateCodeVerifier();
  const codeChallenge = await generateCodeChallenge(codeVerifier);
  const transaction: OAuthTransaction = {
    state,
    codeVerifier,
    nonce,
    createdAt: (config.now ?? Date.now)(),
    returnTo: normalizeReturnTo(input.returnTo),
  };

  const body = new URLSearchParams({
    response_type: 'code',
    client_id: requiredValue(config.clientId, 'OAuth client ID'),
    redirect_uri: requiredValue(config.redirectUri, 'OAuth redirect URI'),
    scope,
    resource,
    state,
    code_challenge: codeChallenge,
    code_challenge_method: 'S256',
  });
  if (nonce) {
    body.set('nonce', nonce);
  }

  const keyStore = config.dpopStore ?? new IndexedDbDpopTransactionStore();
  const key = config.dpop === false ? undefined : await generateBrowserDpopKeyPair();
  if (key) {
    const keyId = crypto.randomUUID();
    await keyStore.save(keyId, key, Date.now() + 15 * 60_000);
    transaction.dpop = {
      keyId,
      jkt: key.jkt,
      issuer: normalizedBaseUrl(config.baseUrl),
      clientId: config.clientId,
      redirectUri: config.redirectUri,
    };
  }
  try {
    if (config.storage.getTransaction())
      throw new Error('An OAuth transaction is already pending.');
    config.storage.saveTransaction(transaction);
    const par = await pushAuthorizationRequest(config, body, key);
    const authorizeUrl = new URL('/oauth/authorize', normalizedBaseUrl(config.baseUrl));
    authorizeUrl.searchParams.set('client_id', config.clientId);
    authorizeUrl.searchParams.set('request_uri', par.requestUri);
    return { authorizationUrl: authorizeUrl.toString(), state };
  } catch (error) {
    if (config.storage.getTransaction()?.codeVerifier === codeVerifier)
      config.storage.clearTransaction();
    if (transaction.dpop) await keyStore.remove(transaction.dpop.keyId);
    throw error;
  }
}

async function pushAuthorizationRequest(
  config: AuthorizationRequestConfig,
  body: URLSearchParams,
  key?: DpopMainKeyPair,
): Promise<ParResponse> {
  const fetchImpl = config.fetchImpl ?? fetch.bind(globalThis);
  const endpoint = `${normalizedBaseUrl(config.baseUrl)}/oauth/par`;
  const proof = key ? await createDpopProof(key, 'POST', endpoint) : undefined;
  const response = await fetchImpl(endpoint, {
    method: 'POST',
    credentials: 'omit',
    redirect: 'error',
    cache: 'no-store',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/x-www-form-urlencoded',
      ...(proof ? { DPoP: proof } : {}),
    },
    body,
  });
  const payload: unknown = await response.json();
  if (!response.ok) {
    throw new Error(readOAuthError(payload, response.status));
  }
  if (!isRecord(payload)) {
    throw new Error('Identity returned an invalid pushed authorization response.');
  }

  const requestUri = payload.request_uri;
  const expiresIn = payload.expires_in;
  if (
    typeof requestUri !== 'string' ||
    !requestUri.startsWith('urn:ietf:params:oauth:request_uri:') ||
    requestUri.length > 512 ||
    typeof expiresIn !== 'number' ||
    !Number.isSafeInteger(expiresIn) ||
    expiresIn <= 0
  ) {
    throw new Error('Identity returned an invalid pushed authorization response.');
  }
  return { requestUri, expiresIn };
}

function generateRandomValue(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function normalizeReturnTo(value: string | undefined): string {
  if (!value) {
    return '/';
  }
  if (
    !value.startsWith('/') ||
    value.startsWith('//') ||
    value.includes('\\') ||
    Array.from(value).some((character) => character.charCodeAt(0) <= 32)
  ) {
    throw new Error('OAuth returnTo must be a same-origin path.');
  }
  return value;
}

function requiredValue(value: string, label: string): string {
  if (!value.trim()) {
    throw new Error(`${label} is required.`);
  }
  return value;
}

function normalizedBaseUrl(value: string): string {
  const url = new URL(requiredValue(value, 'Identity base URL'));
  const local =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if (
    (url.protocol !== 'https:' && !local) ||
    url.username ||
    url.password ||
    url.pathname !== '/' ||
    url.search ||
    url.hash
  ) {
    throw new Error('Identity base URL must be an HTTPS origin or HTTP loopback origin.');
  }
  return url.origin;
}

function transactionValue(value: string): string {
  if (!/^[\x21-\x7e]{16,512}$/u.test(value)) {
    throw new Error('OAuth state and nonce must contain 16 to 512 visible ASCII characters.');
  }
  return value;
}

function readOAuthError(payload: unknown, status: number): string {
  if (isRecord(payload)) {
    for (const key of ['error_description', 'message', 'error']) {
      const value = payload[key];
      if (typeof value === 'string' && value.trim()) {
        return value;
      }
    }
  }
  return `Identity PAR request failed with status ${status}.`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
