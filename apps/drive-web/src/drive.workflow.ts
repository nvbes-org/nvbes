import { NvbesIdentity } from '@nvbes/identity-sdk';
import { Effect } from 'effect';
import type { DriveMeResponse } from './drive.api';
import { fetchDriveMe, logoutDrive } from './drive.api';
import { clearDriveSession, saveSession, identityClient } from './drive.session';

type OAuthFlow = 'login' | 'register';

interface OAuthStateRecord {
  flow: OAuthFlow;
  state: string;
  codeVerifier: string;
}

export interface CompleteDriveCallbackResult {
  accessToken: string;
  tokenType: string;
  me: DriveMeResponse;
}

const OAUTH_STORAGE_KEY = 'nvbes_drive_oauth_state';

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

function createIdentityClient(flow: OAuthFlow): NvbesIdentity {
  return new NvbesIdentity({
    clientId: identityClientId(),
    redirectUri: `${driveOrigin()}/callback`,
    authorizationUrl: `${identityWebBaseUrl()}${flow === 'login' ? '/login' : '/register'}`,
    tokenUrl: `${identityApiBaseUrl()}/oauth/token`,
    userInfoUrl: `${identityApiBaseUrl()}/oauth/userinfo`,
  });
}

function oauthStorage(): OAuthStateRecord | null {
  const raw = sessionStorage.getItem(OAUTH_STORAGE_KEY);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as OAuthStateRecord;
  } catch {
    sessionStorage.removeItem(OAUTH_STORAGE_KEY);
    return null;
  }
}

function setOauthStorage(record: OAuthStateRecord): void {
  sessionStorage.setItem(OAUTH_STORAGE_KEY, JSON.stringify(record));
}

function clearOauthStorage(): void {
  sessionStorage.removeItem(OAUTH_STORAGE_KEY);
}

function base64UrlEncode(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/u, '');
}

function createState(): string {
  return base64UrlEncode(crypto.getRandomValues(new Uint8Array(24)));
}

async function createPkceChallenge(): Promise<{ codeVerifier: string; codeChallenge: string }> {
  const codeVerifier = base64UrlEncode(crypto.getRandomValues(new Uint8Array(48)));
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(codeVerifier));
  return {
    codeVerifier,
    codeChallenge: base64UrlEncode(new Uint8Array(digest)),
  };
}

async function buildAuthorizationUrl(flow: OAuthFlow): Promise<string> {
  const client = createIdentityClient(flow);
  const { codeVerifier, codeChallenge } = await createPkceChallenge();
  const state = createState();

  setOauthStorage({ flow, state, codeVerifier });

  return client.getAuthorizationUrl(
    'openid profile email offline_access drive:read drive:write',
    state,
    codeChallenge,
    'S256',
  );
}

export function startDriveLoginWorkflow() {
  return Effect.tryPromise({
    try: async () => {
      window.location.assign(await buildAuthorizationUrl('login'));
    },
    catch: (error: unknown) => error,
  });
}

export function startDriveRegisterWorkflow() {
  return Effect.tryPromise({
    try: async () => {
      window.location.assign(await buildAuthorizationUrl('register'));
    },
    catch: (error: unknown) => error,
  });
}

export function completeDriveCallbackWorkflow(searchParams: URLSearchParams) {
  return Effect.tryPromise({
    try: async (): Promise<CompleteDriveCallbackResult> => {
      const code = searchParams.get('code');
      const state = searchParams.get('state');

      if (!code || !state) {
        throw new Error('Missing OAuth callback parameters.');
      }

      const stored = oauthStorage();
      if (!stored || stored.state !== state) {
        clearOauthStorage();
        throw new Error('OAuth state mismatch.');
      }

      const token = await identityClient.exchangeCode(code, stored.codeVerifier);
      const me = await fetchDriveMe(token.accessToken);
      saveSession(
        me.user.id,
        me.user.email,
        me.user.display_name,
        token.accessToken,
        token.refreshToken,
      );
      clearOauthStorage();

      return {
        accessToken: token.accessToken,
        tokenType: token.tokenType,
        me,
      };
    },
    catch: (error: unknown) => error,
  });
}

export function fetchDriveMeWorkflow(accessToken: string) {
  return Effect.tryPromise({
    try: () => fetchDriveMe(accessToken),
    catch: (error: unknown) => error,
  });
}

export function logoutDriveWorkflow(accessToken: string | null) {
  return Effect.tryPromise({
    try: async () => {
      clearDriveSession();
      if (accessToken) {
        await logoutDrive(accessToken).catch(() => undefined);
      }
    },
    catch: (error: unknown) => error,
  });
}
