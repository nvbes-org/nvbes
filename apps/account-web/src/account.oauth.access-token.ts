import type { AuthorizationCodeTokenResponse } from '@nvbes/identity-sdk-web/oauth';

interface AccountAccessToken {
  value: string;
  expiresAt: number;
  scopes: ReadonlySet<string>;
}

type AccountAccessTokenListener = () => void;

let currentToken: AccountAccessToken | null = null;
let expirationTimer: ReturnType<typeof setTimeout> | null = null;
const listeners = new Set<AccountAccessTokenListener>();

export function setAccountAccessToken(
  response: AuthorizationCodeTokenResponse,
  now = Date.now(),
): void {
  if (response.tokenType.toLowerCase() !== 'bearer') {
    throw new Error('Identity a retourné un type de jeton OAuth non pris en charge.');
  }
  if (!Number.isFinite(response.expiresIn) || response.expiresIn <= 0) {
    throw new Error('Identity a retourné une durée de jeton OAuth invalide.');
  }

  currentToken = {
    value: response.accessToken,
    expiresAt: now + response.expiresIn * 1_000,
    scopes: new Set(response.scope.split(/\s+/u).filter(Boolean)),
  };
  scheduleExpiration(currentToken.expiresAt, now);
  notifyListeners();
}

export function getAccountAccessToken(now = Date.now()): string | null {
  if (!currentToken) {
    return null;
  }

  if (currentToken.expiresAt <= now + 5_000) {
    clearAccountAccessToken();
    return null;
  }

  return currentToken.value;
}

export function hasAccountScope(scope: string, now = Date.now()): boolean {
  return getAccountAccessToken(now) !== null && Boolean(currentToken?.scopes.has(scope));
}

export function clearAccountAccessToken(): void {
  if (expirationTimer) {
    clearTimeout(expirationTimer);
    expirationTimer = null;
  }
  if (!currentToken) {
    return;
  }
  currentToken = null;
  notifyListeners();
}

function scheduleExpiration(expiresAt: number, now: number): void {
  if (expirationTimer) {
    clearTimeout(expirationTimer);
  }

  const delay = Math.max(0, expiresAt - now - 5_000);
  expirationTimer = setTimeout(() => {
    expirationTimer = null;
    clearAccountAccessToken();
  }, delay);
}

export function subscribeAccountAccessToken(listener: AccountAccessTokenListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function getAccountAccessTokenSnapshot(): string | null {
  return getAccountAccessToken();
}

function notifyListeners(): void {
  for (const listener of listeners) {
    listener();
  }
}
