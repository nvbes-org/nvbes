import { createAccountClient } from '@nvbes/account-client';
import { getAccountAccessToken } from './account.oauth.access-token';

export const accountClient = createAccountClient({
  baseUrl: normalizedAccountServiceBaseUrl(),
  getAccessToken: getAccountAccessToken,
});

function normalizedAccountServiceBaseUrl(): string {
  const configured = import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL?.trim();
  if (configured) {
    return configured.replace(/\/+$/u, '');
  }
  return globalThis.location?.origin ?? 'http://localhost:3001';
}
