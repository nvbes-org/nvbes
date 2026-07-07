import { NvbesIdentity } from '@nvbes/identity-sdk';

import type { IdentityRuntimeClient } from './drive.session.tokens';

function driveOrigin() {
  return window.location.origin;
}

function accountWebBaseUrl() {
  return (import.meta.env.VITE_ACCOUNT_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}

function accountServiceBaseUrl() {
  return (import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL || 'http://localhost:8080').replace(
    /\/+$/u,
    '',
  );
}

function identityClientId() {
  const clientId = import.meta.env.VITE_ACCOUNT_CLIENT_ID;
  if (!clientId) {
    throw new Error('VITE_ACCOUNT_CLIENT_ID is required.');
  }
  return clientId;
}

export const identityClient = new NvbesIdentity({
  clientId: identityClientId(),
  redirectUri: `${driveOrigin()}/callback`,
  authorizationUrl: `${accountWebBaseUrl()}/login`,
  tokenUrl: `${accountServiceBaseUrl()}/oauth/token`,
  userInfoUrl: `${accountServiceBaseUrl()}/oauth/userinfo`,
});

export const identityRuntimeClient = identityClient as unknown as IdentityRuntimeClient;

export interface AccountSession {
  userId: string;
  email: string;
  name: string;
  accessToken: string;
  refreshToken?: string;
}

export function setIdentityClientToken(session: AccountSession): void {
  identityRuntimeClient.token = {
    accessToken: session.accessToken,
    tokenType: 'Bearer',
    expiresIn: 3600,
    refreshToken: session.refreshToken,
    scope: 'openid profile email offline_access drive:read drive:write',
  };
}
