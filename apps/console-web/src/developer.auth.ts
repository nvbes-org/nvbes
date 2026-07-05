import { normalizeClientError } from '@nvbes/web-runtime';

import { developerIdentityClient, developerSessionFromToken } from './developer.oauth.client';
import {
  buildDeveloperAuthorizationUrl,
  clearDeveloperOauthState,
  readDeveloperOauthState,
} from './developer.oauth.state';
import { clearDeveloperSession, saveDeveloperSession } from './developer.session.storage';

export async function startDeveloperLogin(returnTo: string = window.location.href): Promise<void> {
  window.location.assign(await buildDeveloperAuthorizationUrl(returnTo));
}

export async function completeDeveloperCallback(searchParams: URLSearchParams): Promise<string> {
  try {
    const code = searchParams.get('code');
    const state = searchParams.get('state');
    if (!code || !state) {
      throw new Error('Missing OAuth callback parameters.');
    }

    const stored = readDeveloperOauthState();
    if (!stored || stored.state !== state) {
      clearDeveloperOauthState();
      throw new Error('OAuth state mismatch.');
    }

    const token = await developerIdentityClient.exchangeCode(code, stored.codeVerifier);
    saveDeveloperSession(developerSessionFromToken(token));
    clearDeveloperOauthState();
    return stored.returnTo || '/portal/apps';
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export function clearDeveloperAuth(): void {
  clearDeveloperSession();
  developerIdentityClient.logout();
}
