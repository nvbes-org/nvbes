import { createHttpClient } from '@nvbes/http-client';
import { verifiedFetch } from '@nvbes/web-runtime';
import { getDeveloperAccessToken } from './developer.session.storage';

export const accountServiceBaseUrl =
  import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
  globalThis.location?.origin ||
  'http://localhost:4000';

export const developerServiceBaseUrl =
  import.meta.env.VITE_DEVELOPER_SERVICE_BASE_URL ||
  globalThis.location?.origin ||
  'http://localhost:4040';

function authenticatedFetch(allowedOrigin: string): typeof fetch {
  return (input, init) => {
    const headers = new Headers(init?.headers);
    const token = getDeveloperAccessToken();
    if (token && !headers.has('Authorization')) {
      headers.set('Authorization', `Bearer ${token}`);
    }

    return verifiedFetch(input, {
      ...init,
      allowedOrigins: [allowedOrigin],
      credentials: init?.credentials ?? 'include',
      headers,
    });
  };
}

export const identityVerifiedFetch = authenticatedFetch(accountServiceBaseUrl);
export const developerVerifiedFetch = authenticatedFetch(developerServiceBaseUrl);

export const identityHttpClient = createHttpClient({
  baseUrl: accountServiceBaseUrl,
  credentials: 'include',
  fetchImpl: identityVerifiedFetch,
});

export const developerHttpClient = createHttpClient({
  baseUrl: developerServiceBaseUrl,
  credentials: 'omit',
  fetchImpl: developerVerifiedFetch,
});
