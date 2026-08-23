import { getSafeSessionStorage } from '@nvbes/web-runtime';
import { z } from 'zod';
import { identityHttpClient } from './identity.http';

export interface OAuthAuthorizeRequest {
  clientId: string;
  redirectUri: string;
  state?: string | null;
  scope?: string | null;
  codeChallenge?: string | null;
  codeChallengeMethod?: string | null;
  nonce?: string | null;
  consentAction?: string | null;
}

const ParResponseSchema = z.object({
  request_uri: z.string(),
  expires_in: z.number(),
});

const PENDING_OAUTH_KEY = 'nvbes.pending_oauth_authorize';
const OAuthAuthorizeRequestSchema = z.object({
  clientId: z.string(),
  codeChallenge: z.string().nullable().optional(),
  codeChallengeMethod: z.string().nullable().optional(),
  consentAction: z.string().nullable().optional(),
  redirectUri: z.string(),
  scope: z.string().nullable().optional(),
  state: z.string().nullable().optional(),
  nonce: z.string().nullable().optional(),
});

function oauthApiUrl(): string {
  return '/oauth/authorize';
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
    nonce: searchParams.get('nonce'),
    consentAction: searchParams.get('consent_action'),
  };
}

export function savePendingOAuthAuthorizeRequest(request: OAuthAuthorizeRequest): void {
  getSafeSessionStorage().setJson(PENDING_OAUTH_KEY, request);
}

export function readPendingOAuthAuthorizeRequest(): OAuthAuthorizeRequest | null {
  const request = getSafeSessionStorage().getJson(PENDING_OAUTH_KEY, OAuthAuthorizeRequestSchema);
  if (!request) {
    clearPendingOAuthAuthorizeRequest();
  }
  return request;
}

export function clearPendingOAuthAuthorizeRequest(): void {
  getSafeSessionStorage().removeItem(PENDING_OAUTH_KEY);
}

export async function authorizeIdentitySession(
  _legacySessionToken: string | null | undefined,
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

  if (request.nonce) {
    parBody.set('nonce', request.nonce);
  }

  if (request.consentAction) {
    parBody.set('consent_action', request.consentAction);
  }

  const parData = await identityHttpClient.post('/oauth/par', ParResponseSchema, parBody, {
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
  });

  const url = new URL(oauthApiUrl(), window.location.origin);
  url.searchParams.set('response_type', 'code');
  url.searchParams.set('client_id', request.clientId);
  url.searchParams.set('request_uri', parData.request_uri);

  window.location.assign(url.toString());
}
