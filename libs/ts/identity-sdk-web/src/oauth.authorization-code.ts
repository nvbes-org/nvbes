import type { WebStorage } from './storage';
import { verifyIdToken, type VerifiedIdentity } from './oidc.id-token';
import { createDpopProof, type DpopMainKeyPair } from './dpop';
import { IndexedDbDpopTransactionStore, type DpopTransactionStore } from './dpop.transaction-store';

const AUTHORIZATION_TRANSACTION_MAX_AGE_MS = 15 * 60 * 1_000;

export interface AuthorizationCodeTokenResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
  refreshToken: string | null;
  idToken: string | null;
  scope: string;
  returnTo: string;
  dpopKey?: DpopMainKeyPair;
  identity?: VerifiedIdentity;
}

export interface ExchangeAuthorizationCodeInput {
  code: string;
  state: string;
}

export interface ExchangeAuthorizationCodeConfig {
  baseUrl: string;
  clientId: string;
  redirectUri: string;
  storage: WebStorage;
  fetchImpl?: typeof fetch;
  now?: () => number;
  dpopStore?: DpopTransactionStore;
}

export async function exchangeAuthorizationCode(
  config: ExchangeAuthorizationCodeConfig,
  input: ExchangeAuthorizationCodeInput,
): Promise<AuthorizationCodeTokenResponse> {
  const transaction = config.storage.getTransaction();
  if (!transaction || !input.state || input.state !== transaction.state) {
    throw new Error('OAuth state validation failed.');
  }

  const age = (config.now ?? Date.now)() - transaction.createdAt;
  if (age < 0 || age > AUTHORIZATION_TRANSACTION_MAX_AGE_MS) {
    config.storage.clearTransaction();
    throw new Error('OAuth authorization transaction expired.');
  }

  const endpoint = `${config.baseUrl.replace(/\/+$/u, '')}/oauth/token`;
  const keyStore = config.dpopStore ?? new IndexedDbDpopTransactionStore();
  let key: DpopMainKeyPair | undefined;
  if (transaction.dpop) {
    const binding = transaction.dpop;
    if (
      binding.issuer !== new URL(config.baseUrl).origin ||
      binding.clientId !== config.clientId ||
      binding.redirectUri !== config.redirectUri
    )
      throw new Error('OAuth DPoP transaction context mismatch.');
    key = (await keyStore.load(binding.keyId)) ?? undefined;
    if (!key || key.jkt !== binding.jkt)
      throw new Error('OAuth DPoP transaction key missing or mismatched.');
  }
  const proof = key ? await createDpopProof(key, 'POST', endpoint) : undefined;
  const body = new URLSearchParams({
    grant_type: 'authorization_code',
    client_id: config.clientId,
    code: input.code,
    redirect_uri: config.redirectUri,
    code_verifier: transaction.codeVerifier,
  });
  const fetchImpl = config.fetchImpl ?? fetch.bind(globalThis);
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

  const tokens = parseTokenResponse(payload);
  if (tokens.tokenType !== (key ? 'DPoP' : 'Bearer'))
    throw new Error('Unexpected OAuth token binding.');
  const identity = await verifyIdToken({
    token: tokens.idToken,
    accessToken: tokens.accessToken,
    issuer: config.baseUrl,
    clientId: config.clientId,
    nonce: transaction.nonce,
    fetchImpl: config.fetchImpl,
    now: config.now,
  });
  if (transaction.dpop) await keyStore.remove(transaction.dpop.keyId);
  config.storage.clearTransaction();
  return { ...tokens, identity, returnTo: transaction.returnTo, ...(key ? { dpopKey: key } : {}) };
}

export function parseTokenResponse(
  payload: unknown,
): Omit<AuthorizationCodeTokenResponse, 'returnTo'> {
  if (!isRecord(payload)) {
    throw new Error('Identity returned an invalid OAuth token response.');
  }

  const accessToken = requiredString(payload, 'access_token');
  const tokenType = requiredString(payload, 'token_type');
  const scope = requiredString(payload, 'scope');
  const expiresIn = payload.expires_in;
  if (typeof expiresIn !== 'number' || !Number.isSafeInteger(expiresIn) || expiresIn <= 0) {
    throw new Error('Identity returned an invalid OAuth token lifetime.');
  }

  return {
    accessToken,
    tokenType,
    expiresIn,
    refreshToken: optionalString(payload, 'refresh_token'),
    idToken: optionalString(payload, 'id_token'),
    scope,
  };
}

function requiredString(payload: Record<string, unknown>, key: string): string {
  const value = payload[key];
  if (typeof value !== 'string' || !value.trim()) {
    throw new Error(`Identity OAuth response is missing ${key}.`);
  }
  return value;
}

function optionalString(payload: Record<string, unknown>, key: string): string | null {
  const value = payload[key];
  return typeof value === 'string' && value.trim() ? value : null;
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
  return `Identity OAuth token exchange failed with status ${status}.`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
