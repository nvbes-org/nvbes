import { NvbesIdentity } from '@nvbes/identity-sdk';

export type OAuthFlow = 'login' | 'register';

function driveOrigin(): string {
  return window.location.origin;
}

function accountWebBaseUrl(): string {
  return (import.meta.env.VITE_ACCOUNT_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}

function accountServiceBaseUrl(): string {
  return (import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL || 'http://localhost:8080').replace(
    /\/+$/u,
    '',
  );
}

function identityClientId(): string {
  const clientId = import.meta.env.VITE_ACCOUNT_CLIENT_ID;
  if (!clientId) {
    throw new Error('VITE_ACCOUNT_CLIENT_ID is required.');
  }

  return clientId;
}

export function createIdentityClient(flow: OAuthFlow): NvbesIdentity {
  return new NvbesIdentity({
    clientId: identityClientId(),
    redirectUri: `${driveOrigin()}/callback`,
    authorizationUrl: `${accountWebBaseUrl()}${flow === 'login' ? '/login' : '/register'}`,
    tokenUrl: `${accountServiceBaseUrl()}/oauth/token`,
    userInfoUrl: `${accountServiceBaseUrl()}/oauth/userinfo`,
  });
}
