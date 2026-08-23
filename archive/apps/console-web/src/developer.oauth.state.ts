import { getSafeSessionStorage } from '@nvbes/web-runtime';
import { z } from 'zod';

type DeveloperOAuthState = {
  state: string;
  codeVerifier: string;
  nonce: string;
  returnTo: string;
};

const OAUTH_STORAGE_KEY = 'nvbes_developer_oauth_state';
const DeveloperOAuthStateSchema = z.object({
  codeVerifier: z.string(),
  returnTo: z.string(),
  state: z.string(),
  nonce: z.string(),
});

export function readDeveloperOauthState(): DeveloperOAuthState | null {
  const state = getSafeSessionStorage().getJson(OAUTH_STORAGE_KEY, DeveloperOAuthStateSchema);
  if (!state) {
    clearDeveloperOauthState();
  }
  return state;
}

export function clearDeveloperOauthState(): void {
  getSafeSessionStorage().removeItem(OAUTH_STORAGE_KEY);
}

export async function buildDeveloperAuthorizationUrl(returnTo: string): Promise<string> {
  const { codeVerifier, codeChallenge } = await createPkceChallenge();
  const state = createRandomValue(24);
  const nonce = createRandomValue(24);
  getSafeSessionStorage().setJson(OAUTH_STORAGE_KEY, { state, codeVerifier, nonce, returnTo });

  const { developerIdentityClient } = await import('./developer.oauth.client');
  return developerIdentityClient.getAuthorizationUrl(
    'openid profile email offline_access',
    state,
    codeChallenge,
    nonce,
  );
}

async function createPkceChallenge(): Promise<{ codeVerifier: string; codeChallenge: string }> {
  const codeVerifier = createRandomValue(48);
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(codeVerifier));

  return {
    codeVerifier,
    codeChallenge: base64UrlEncode(new Uint8Array(digest)),
  };
}

function createRandomValue(length: number): string {
  return base64UrlEncode(crypto.getRandomValues(new Uint8Array(length)));
}

function base64UrlEncode(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }

  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/u, '');
}
