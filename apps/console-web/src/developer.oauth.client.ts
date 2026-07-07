import { NvbesIdentity } from '@nvbes/identity-sdk';
import type { TokenResponse } from '@nvbes/identity-sdk';

import type { DeveloperSession } from './developer.session.storage';

function developerOrigin(): string {
  return window.location.origin;
}

function accountWebBaseUrl(): string {
  return (import.meta.env.VITE_ACCOUNT_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}

function accountServiceBaseUrl(): string {
  return (import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL || window.location.origin).replace(
    /\/+$/u,
    '',
  );
}

function identityClientId(): string {
  return (
    import.meta.env.VITE_CONSOLE_ACCOUNT_CLIENT_ID ||
    import.meta.env.VITE_ACCOUNT_CLIENT_ID ||
    'console-web'
  );
}

export const developerIdentityClient = new NvbesIdentity({
  clientId: identityClientId(),
  redirectUri: `${developerOrigin()}/callback`,
  authorizationUrl: `${accountWebBaseUrl()}/login`,
  tokenUrl: `${accountServiceBaseUrl()}/oauth/token`,
  userInfoUrl: `${accountServiceBaseUrl()}/oauth/userinfo`,
});

export function developerSessionFromToken(token: TokenResponse): DeveloperSession {
  return {
    accessToken: token.accessToken,
    refreshToken: token.refreshToken,
    scope: token.scope,
    tokenType: token.tokenType,
  };
}
