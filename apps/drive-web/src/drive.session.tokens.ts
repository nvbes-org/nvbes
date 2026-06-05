import type { IdentityConfig, TokenResponse } from '@nvbes/identity-sdk';

export interface IdentityRuntimeClient {
  config: Pick<IdentityConfig, 'clientId' | 'tokenUrl'>;
  token: TokenResponse | null;
}

interface RefreshTokenResponse {
  accessToken: string;
  refreshToken?: string;
}

interface OAuthRefreshResponse {
  access_token?: unknown;
  refresh_token?: unknown;
}

class TokenRefreshError extends Error {
  readonly status: number;

  constructor(status: number, statusText: string) {
    super(`Failed to refresh token: ${statusText}`);
    this.name = 'TokenRefreshError';
    this.status = status;
  }
}

export function isJwtExpired(token: string): boolean {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return true;
    const payload = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/')));
    const exp = payload.exp;
    if (typeof exp !== 'number') return true;
    return Date.now() / 1000 >= exp - 10;
  } catch {
    return true;
  }
}

function getErrorStatus(error: unknown): number | null {
  if (!error || typeof error !== 'object') {
    return null;
  }

  const status = 'status' in error ? error.status : undefined;
  if (typeof status === 'number') {
    return status;
  }

  const cause = 'cause' in error ? error.cause : undefined;
  if (cause && typeof cause === 'object' && 'status' in cause && typeof cause.status === 'number') {
    return cause.status;
  }

  return null;
}

export function isUnauthorizedSessionError(error: unknown): boolean {
  const status = getErrorStatus(error);
  return status === 400 || status === 401;
}

export async function refreshAccessToken(
  client: IdentityRuntimeClient,
  refreshToken: string,
): Promise<RefreshTokenResponse> {
  const response = await fetch(client.config.tokenUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: new URLSearchParams({
      grant_type: 'refresh_token',
      refresh_token: refreshToken,
      client_id: client.config.clientId,
    }),
  });
  if (!response.ok) {
    throw new TokenRefreshError(response.status, response.statusText);
  }

  const data = (await response.json()) as OAuthRefreshResponse;
  if (typeof data.access_token !== 'string') {
    throw new Error('Refresh response did not include an access token.');
  }

  return {
    accessToken: data.access_token,
    refreshToken: typeof data.refresh_token === 'string' ? data.refresh_token : undefined,
  };
}
