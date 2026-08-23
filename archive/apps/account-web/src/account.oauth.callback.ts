import { setAccountAccessToken } from './account.oauth.access-token';
import {
  assertGrantedAccountScopes,
  parseAccountOAuthCallback,
} from './account.oauth.callback-params';
import {
  ACCOUNT_OAUTH_SCOPES,
  clearAccountAuthorizationTransaction,
  exchangeAccountAuthorizationCode,
} from './account.oauth.client';

let callbackInFlight: Promise<string> | null = null;

export function completeAccountOAuthCallback(search = window.location.search): Promise<string> {
  callbackInFlight ??= exchangeCallback(search).catch((error: unknown) => {
    callbackInFlight = null;
    throw error;
  });
  return callbackInFlight;
}

async function exchangeCallback(search: string): Promise<string> {
  let callback: { code: string; state: string };
  try {
    callback = parseAccountOAuthCallback(search);
  } catch (error) {
    clearAccountAuthorizationTransaction();
    throw error;
  }

  const response = await exchangeAccountAuthorizationCode(callback);
  try {
    assertGrantedAccountScopes(response.scope, ACCOUNT_OAUTH_SCOPES);
  } catch (error) {
    clearAccountAuthorizationTransaction();
    throw error;
  }
  setAccountAccessToken(response);
  return response.returnTo;
}
