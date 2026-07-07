import { createHttpClient } from '@nvbes/http-client';
import { verifiedFetch } from '@nvbes/web-runtime';
import { getDeveloperAccessToken } from './developer.session.storage';

export const accountServiceBaseUrl =
  import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
  globalThis.location?.origin ||
  'http://localhost:4000';

export const identityVerifiedFetch: typeof fetch = (input, init) => {
  const headers = new Headers(init?.headers);
  const token = getDeveloperAccessToken();
  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  return verifiedFetch(input, {
    ...init,
    allowedOrigins: [accountServiceBaseUrl],
    credentials: init?.credentials ?? 'include',
    headers,
  });
};

export const identityHttpClient = createHttpClient({
  baseUrl: accountServiceBaseUrl,
  credentials: 'include',
  fetchImpl: identityVerifiedFetch,
});
