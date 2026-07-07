import { createHttpClient } from '@nvbes/http-client';

export const accountServiceBaseUrl =
  import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL || 'http://localhost:4000';

export const identityHttpClient = createHttpClient({
  baseUrl: accountServiceBaseUrl,
  credentials: 'include',
});
