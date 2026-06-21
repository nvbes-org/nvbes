import { createHttpClient } from '@nvbes/http-client';
import { getDeveloperAccessToken } from './developer.session.storage';

export const identityApiBaseUrl =
  import.meta.env.VITE_IDENTITY_API_BASE_URL ||
  globalThis.location?.origin ||
  'http://localhost:4000';

export const identityHttpClient = createHttpClient({
  baseUrl: identityApiBaseUrl,
  credentials: 'include',
  headers: developerAuthHeaders(),
});

function developerAuthHeaders(): HeadersInit | undefined {
  const token = getDeveloperAccessToken();
  return token ? { Authorization: `Bearer ${token}` } : undefined;
}
