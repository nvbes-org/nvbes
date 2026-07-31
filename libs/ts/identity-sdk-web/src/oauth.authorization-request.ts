import { generateCodeChallenge, generateCodeVerifier } from './pkce';
import type { OAuthTransaction, WebStorage } from './storage';

export interface AuthorizationRequestConfig {
  baseUrl: string;
  clientId: string;
  redirectUri: string;
  audience?: string;
  storage: WebStorage;
  fetchImpl?: typeof fetch;
  now?: () => number;
}

export interface AuthorizationRequestInput {
  scope?: string;
  audience?: string;
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
  const state = input.state ?? generateRandomValue();
  const scope = input.scope?.trim() || 'openid profile email';
  const requiresNonce = scope.split(/\s+/u).includes('openid');
  const nonce = requiresNonce ? (input.nonce ?? generateRandomValue()) : null;
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
    state,
    code_challenge: codeChallenge,
    code_challenge_method: 'S256',
  });
  if (nonce) {
    body.set('nonce', nonce);
  }

  const audience = input.audience?.trim() || config.audience?.trim();
  if (audience) {
    body.set('audience', audience);
  }

  config.storage.saveTransaction(transaction);
  try {
    const par = await pushAuthorizationRequest(config, body);
    const authorizeUrl = new URL('/oauth/authorize', normalizedBaseUrl(config.baseUrl));
    authorizeUrl.searchParams.set('client_id', config.clientId);
    authorizeUrl.searchParams.set('request_uri', par.requestUri);
    return { authorizationUrl: authorizeUrl.toString(), state };
  } catch (error) {
    config.storage.clearTransaction();
    throw error;
  }
}

async function pushAuthorizationRequest(
  config: AuthorizationRequestConfig,
  body: URLSearchParams,
): Promise<ParResponse> {
  const fetchImpl = config.fetchImpl ?? fetch.bind(globalThis);
  const response = await fetchImpl(`${normalizedBaseUrl(config.baseUrl)}/oauth/par`, {
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
  if (!isRecord(payload)) {
    throw new Error('Identity returned an invalid pushed authorization response.');
  }

  const requestUri = payload.request_uri;
  const expiresIn = payload.expires_in;
  if (
    typeof requestUri !== 'string' ||
    !requestUri.trim() ||
    typeof expiresIn !== 'number' ||
    !Number.isFinite(expiresIn) ||
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
  if (!value.startsWith('/') || value.startsWith('//')) {
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
  return requiredValue(value, 'Identity base URL').replace(/\/+$/u, '');
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
