import { NvbesIdentity } from '@nvbes/identity-sdk';
import type { TokenResponse } from '@nvbes/identity-sdk';

import type { DeveloperSession } from './developer.session.storage';

function developerOrigin(): string {
  return window.location.origin;
}

function identityWebBaseUrl(): string {
  return (import.meta.env.VITE_IDENTITY_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}

function identityApiBaseUrl(): string {
  return (import.meta.env.VITE_IDENTITY_API_BASE_URL || window.location.origin).replace(
    /\/+$/u,
    '',
  );
}

function identityClientId(): string {
  return (
    import.meta.env.VITE_DEVELOPER_IDENTITY_CLIENT_ID ||
    import.meta.env.VITE_IDENTITY_CLIENT_ID ||
    'developer-web'
  );
}

export const developerIdentityClient = new NvbesIdentity({
  clientId: identityClientId(),
  redirectUri: `${developerOrigin()}/callback`,
  authorizationUrl: `${identityWebBaseUrl()}/login`,
  tokenUrl: `${identityApiBaseUrl()}/oauth/token`,
  userInfoUrl: `${identityApiBaseUrl()}/oauth/userinfo`,
});

export function developerSessionFromToken(token: TokenResponse): DeveloperSession {
  return {
    accessToken: token.accessToken,
    refreshToken: token.refreshToken,
    scope: token.scope,
    tokenType: token.tokenType,
  };
}
