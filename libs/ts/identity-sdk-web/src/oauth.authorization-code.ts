import type { WebStorage } from './storage';

const AUTHORIZATION_TRANSACTION_MAX_AGE_MS = 15 * 60 * 1_000;

export interface AuthorizationCodeTokenResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
  refreshToken: string | null;
  idToken: string | null;
  scope: string;
  returnTo: string;
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

  const body = new URLSearchParams({
    grant_type: 'authorization_code',
    client_id: config.clientId,
    code: input.code,
    redirect_uri: config.redirectUri,
    code_verifier: transaction.codeVerifier,
  });
  const fetchImpl = config.fetchImpl ?? fetch.bind(globalThis);
  const response = await fetchImpl(`${config.baseUrl.replace(/\/+$/u, '')}/oauth/token`, {
    method: 'POST',
    credentials: 'omit',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body,
  });
  const payload: unknown = await response.json();
  if (!response.ok) {
    throw new Error(readOAuthError(payload, response.status));
  }

  const tokens = parseTokenResponse(payload);
  config.storage.clearTransaction();
  return { ...tokens, returnTo: transaction.returnTo };
}

function parseTokenResponse(payload: unknown): Omit<AuthorizationCodeTokenResponse, 'returnTo'> {
  if (!isRecord(payload)) {
    throw new Error('Identity returned an invalid OAuth token response.');
  }

  const accessToken = requiredString(payload, 'access_token');
  const tokenType = requiredString(payload, 'token_type');
  const scope = requiredString(payload, 'scope');
  const expiresIn = payload.expires_in;
  if (typeof expiresIn !== 'number' || !Number.isFinite(expiresIn) || expiresIn <= 0) {
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
