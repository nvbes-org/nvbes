import { createHttpClient } from '@nvbes/http-client';

export const identityApiBaseUrl =
  import.meta.env.VITE_IDENTITY_API_BASE_URL || window.location.origin;

export const identityHttpClient = createHttpClient({
  baseUrl: identityApiBaseUrl,
  credentials: 'include',
});
