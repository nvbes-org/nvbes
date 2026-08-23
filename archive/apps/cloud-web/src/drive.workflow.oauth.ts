import { getSafeSessionStorage } from '@nvbes/web-runtime';
import { z } from 'zod';
import type { OAuthFlow } from './drive.workflow.config';
import { createIdentityClient } from './drive.workflow.config';

interface OAuthStateRecord {
  flow: OAuthFlow;
  state: string;
  codeVerifier: string;
  nonce: string;
}

const OAUTH_STORAGE_KEY = 'nvbes_drive_oauth_state';
const OAuthStateRecordSchema = z.object({
  codeVerifier: z.string(),
  flow: z.enum(['login', 'register']),
  state: z.string(),
  nonce: z.string(),
});

function oauthStorage(): OAuthStateRecord | null {
  const record = getSafeSessionStorage().getJson(OAUTH_STORAGE_KEY, OAuthStateRecordSchema);
  if (!record) {
    clearOauthStorage();
  }
  return record;
}

function setOauthStorage(record: OAuthStateRecord): void {
  getSafeSessionStorage().setJson(OAUTH_STORAGE_KEY, record);
}

export function clearOauthStorage(): void {
  getSafeSessionStorage().removeItem(OAUTH_STORAGE_KEY);
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

export async function buildAuthorizationUrl(flow: OAuthFlow): Promise<string> {
  const client = createIdentityClient(flow);
  const { codeVerifier, codeChallenge } = await createPkceChallenge();
  const state = createState();
  const nonce = createState();

  setOauthStorage({ flow, state, codeVerifier, nonce });

  return client.getAuthorizationUrl(
    'openid profile email offline_access drive:read drive:write',
    state,
    codeChallenge,
    nonce,
  );
}

export function readOauthStorage(): OAuthStateRecord | null {
  return oauthStorage();
}
