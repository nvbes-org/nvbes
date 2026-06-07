import { NvbesIdentity } from '@nvbes/identity-sdk';

export type OAuthFlow = 'login' | 'register';

function driveOrigin(): string {
  return window.location.origin;
}

function identityWebBaseUrl(): string {
  return (import.meta.env.VITE_IDENTITY_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}

function identityApiBaseUrl(): string {
  return (import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:8080').replace(
    /\/+$/u,
    '',
  );
}

function identityClientId(): string {
  const clientId = import.meta.env.VITE_IDENTITY_CLIENT_ID;
  if (!clientId) {
    throw new Error('VITE_IDENTITY_CLIENT_ID is required.');
  }

  return clientId;
}

export function createIdentityClient(flow: OAuthFlow): NvbesIdentity {
  return new NvbesIdentity({
    clientId: identityClientId(),
    redirectUri: `${driveOrigin()}/callback`,
    authorizationUrl: `${identityWebBaseUrl()}${flow === 'login' ? '/login' : '/register'}`,
    tokenUrl: `${identityApiBaseUrl()}/oauth/token`,
    userInfoUrl: `${identityApiBaseUrl()}/oauth/userinfo`,
  });
}
