import { createHttpClient } from '@nvbes/http-client';
import { verifiedFetch } from '@nvbes/web-runtime';
import { getDeveloperAccessToken } from './developer.session.storage';

export const identityApiBaseUrl =
  import.meta.env.VITE_IDENTITY_API_BASE_URL ||
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
    allowedOrigins: [identityApiBaseUrl],
    credentials: init?.credentials ?? 'include',
    headers,
  });
};

export const identityHttpClient = createHttpClient({
  baseUrl: identityApiBaseUrl,
  credentials: 'include',
  fetchImpl: identityVerifiedFetch,
});
