import { NvbesIdentity } from '@nvbes/identity-sdk';

import type { IdentityRuntimeClient } from './drive.session.tokens';

function driveOrigin() {
  return window.location.origin;
}

function identityWebBaseUrl() {
  return (import.meta.env.VITE_IDENTITY_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}

function identityApiBaseUrl() {
  return (import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:8080').replace(
    /\/+$/u,
    '',
  );
}

function identityClientId() {
  const clientId = import.meta.env.VITE_IDENTITY_CLIENT_ID;
  if (!clientId) {
    throw new Error('VITE_IDENTITY_CLIENT_ID is required.');
  }
  return clientId;
}

export const identityClient = new NvbesIdentity({
  clientId: identityClientId(),
  redirectUri: `${driveOrigin()}/callback`,
  authorizationUrl: `${identityWebBaseUrl()}/login`,
  tokenUrl: `${identityApiBaseUrl()}/oauth/token`,
  userInfoUrl: `${identityApiBaseUrl()}/oauth/userinfo`,
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
