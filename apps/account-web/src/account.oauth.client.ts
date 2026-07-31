import {
  createAuthorizationRequest,
  defaultWebStorage,
  exchangeAuthorizationCode,
  type AuthorizationCodeTokenResponse,
} from '@nvbes/identity-sdk-web/oauth';

export const ACCOUNT_OAUTH_SCOPES = [
  'account:profile:read',
  'account:profile:write',
  'account:preferences:read',
  'account:preferences:write',
  'account:legal:read',
  'account:legal:write',
  'account:export',
  'account:delete',
] as const;

const identityServiceBaseUrl = readRequiredEnv(
  import.meta.env.VITE_IDENTITY_SERVICE_BASE_URL,
  'VITE_IDENTITY_SERVICE_BASE_URL',
);
const clientId = readRequiredEnv(
  import.meta.env.VITE_ACCOUNT_OAUTH_CLIENT_ID,
  'VITE_ACCOUNT_OAUTH_CLIENT_ID',
);
const redirectUri = readRequiredEnv(
  import.meta.env.VITE_ACCOUNT_OAUTH_REDIRECT_URI,
  'VITE_ACCOUNT_OAUTH_REDIRECT_URI',
);
const oauthStorage = defaultWebStorage();

let authorizationInFlight: Promise<void> | null = null;

export function startAccountAuthorization(returnTo = '/profile'): Promise<void> {
  authorizationInFlight ??= redirectToIdentity(returnTo).catch((error: unknown) => {
    authorizationInFlight = null;
    throw error;
  });
  return authorizationInFlight;
}

export function exchangeAccountAuthorizationCode(input: {
  code: string;
  state: string;
}): Promise<AuthorizationCodeTokenResponse> {
  return exchangeAuthorizationCode(
    {
      baseUrl: identityServiceBaseUrl,
      clientId,
      redirectUri,
      storage: oauthStorage,
    },
    input,
  );
}

export function clearAccountAuthorizationTransaction(): void {
  oauthStorage.clearTransaction();
}

async function redirectToIdentity(returnTo: string): Promise<void> {
  const request = await createAuthorizationRequest(
    {
      baseUrl: identityServiceBaseUrl,
      clientId,
      redirectUri,
      audience: 'nvbes-account-service',
      storage: oauthStorage,
    },
    {
      scope: ACCOUNT_OAUTH_SCOPES.join(' '),
      returnTo,
    },
  );
  window.location.assign(request.authorizationUrl);
}

function readRequiredEnv(value: string | undefined, name: string): string {
  const normalized = value?.trim();
  if (!normalized) {
    throw new Error(`${name} is required for Account OAuth.`);
  }
  return normalized;
}
