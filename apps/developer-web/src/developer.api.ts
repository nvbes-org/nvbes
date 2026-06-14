import { createHttpClient } from '@nvbes/http-client';

import {
  DeveloperAppsResponseSchema,
  DeveloperMeSchema,
  DeveloperWebhooksResponseSchema,
  type DeveloperMe,
  type DeveloperPermission,
} from './developer.schemas';

const developerHttpClient = createHttpClient({
  credentials: 'include',
});

export function getDeveloperMe(options?: { signal?: AbortSignal }) {
  return developerHttpClient.get('/developer/me', DeveloperMeSchema, options);
}

export function listDeveloperApps(options?: { signal?: AbortSignal }) {
  return developerHttpClient
    .get('/developer/apps', DeveloperAppsResponseSchema, options)
    .then((response) => response.apps);
}

export function listDeveloperWebhooks(options?: { signal?: AbortSignal }) {
  return developerHttpClient
    .get('/developer/webhooks', DeveloperWebhooksResponseSchema, options)
    .then((response) => response.endpoints);
}

export function hasDeveloperPermission(
  me: DeveloperMe | undefined,
  permission: DeveloperPermission,
): boolean {
  return Boolean(me?.permissions.includes(permission));
}
