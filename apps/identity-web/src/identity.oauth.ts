import { z } from 'zod';
import { identityHttpClient } from './identity.http';

export interface OAuthAuthorizeRequest {
  clientId: string;
  redirectUri: string;
  state?: string | null;
  scope?: string | null;
  codeChallenge?: string | null;
  codeChallengeMethod?: string | null;
  consentAction?: string | null;
}

const OAuthAuthorizeResponseSchema = z.object({
  code: z.string(),
  redirect_uri: z.string(),
  state: z.string().nullable().optional(),
});

const ParResponseSchema = z.object({
  request_uri: z.string(),
  expires_in: z.number(),
});

const PENDING_OAUTH_KEY = 'nvbes.pending_oauth_authorize';

function oauthApiUrl(): string {
  return '/oauth/authorize';
}

function buildRedirectUrl(redirectUri: string, code: string, state?: string | null): string {
  const url = new URL(redirectUri);
  url.searchParams.set('code', code);
  if (state) {
    url.searchParams.set('state', state);
  }
  return url.toString();
}

export function readOAuthAuthorizeRequest(
  searchParams: URLSearchParams,
): OAuthAuthorizeRequest | null {
  const clientId = searchParams.get('client_id');
  const redirectUri = searchParams.get('redirect_uri');

  if (!clientId || !redirectUri) {
    return null;
  }

  return {
    clientId,
    redirectUri,
    state: searchParams.get('state'),
    scope: searchParams.get('scope'),
    codeChallenge: searchParams.get('code_challenge'),
    codeChallengeMethod: searchParams.get('code_challenge_method'),
    consentAction: searchParams.get('consent_action'),
  };
}

export function savePendingOAuthAuthorizeRequest(request: OAuthAuthorizeRequest): void {
  window.sessionStorage.setItem(PENDING_OAUTH_KEY, JSON.stringify(request));
}

export function readPendingOAuthAuthorizeRequest(): OAuthAuthorizeRequest | null {
  const value = window.sessionStorage.getItem(PENDING_OAUTH_KEY);
  if (!value) {
    return null;
  }

  try {
    return JSON.parse(value) as OAuthAuthorizeRequest;
  } catch {
    window.sessionStorage.removeItem(PENDING_OAUTH_KEY);
    return null;
  }
}

export function clearPendingOAuthAuthorizeRequest(): void {
  window.sessionStorage.removeItem(PENDING_OAUTH_KEY);
}

export async function authorizeIdentitySession(
  sessionToken: string | null | undefined,
  request: OAuthAuthorizeRequest,
): Promise<void> {
  const parBody = new URLSearchParams({
    response_type: 'code',
    client_id: request.clientId,
    redirect_uri: request.redirectUri,
    scope: request.scope ?? 'openid profile email',
  });

  if (request.state) {
    parBody.set('state', request.state);
  }

  if (request.codeChallenge) {
    parBody.set('code_challenge', request.codeChallenge);
    parBody.set('code_challenge_method', request.codeChallengeMethod ?? 'S256');
  }

  if (request.consentAction) {
    parBody.set('consent_action', request.consentAction);
  }

  const parHeaders: Record<string, string> = {
    'Content-Type': 'application/x-www-form-urlencoded',
  };
  if (sessionToken) {
    parHeaders['Authorization'] = `Bearer ${sessionToken}`;
  }

  const parData = await identityHttpClient.post('/oauth/par', ParResponseSchema, parBody, {
    headers: parHeaders,
  });

  const url = new URL(oauthApiUrl(), window.location.origin);
  url.searchParams.set('response_type', 'code');
  url.searchParams.set('client_id', request.clientId);
  url.searchParams.set('request_uri', parData.request_uri);

  const headers: Record<string, string> = {};
  if (sessionToken) {
    headers['Authorization'] = `Bearer ${sessionToken}`;
  }

  const data = await identityHttpClient.get(url.toString(), OAuthAuthorizeResponseSchema, {
    headers,
  });
  window.location.assign(buildRedirectUrl(data.redirect_uri, data.code, data.state));
}
