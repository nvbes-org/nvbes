import { createHttpClient } from '@nvbes/http-client';

export const identityServiceBaseUrl = globalThis.location?.origin ?? 'http://localhost:4000';

export const identityHttpClient = createHttpClient({
  baseUrl: identityServiceBaseUrl,
  credentials: 'include',
});
