import { createHttpClient } from '@nvbes/http-client';

export const accountServiceBaseUrl = globalThis.location?.origin ?? 'http://localhost:4000';

export const identityHttpClient = createHttpClient({
  baseUrl: accountServiceBaseUrl,
  credentials: 'include',
});
