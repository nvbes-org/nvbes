import { createHttpClient } from '@nvbes/http-client';
import { z } from 'zod';

import {
  CreateDeveloperAppResponseSchema,
  CreateDeveloperWebhookEndpointResponseSchema,
  DeveloperAppsResponseSchema,
  DeveloperLogsResponseSchema,
  DeveloperMeSchema,
  DeveloperWebhooksResponseSchema,
  InspectDeveloperTokenResponseSchema,
  OAuthPlaygroundExchangeResponseSchema,
  type DeveloperMe,
  type DeveloperPermission,
} from './developer.schemas';

const developerHttpClient = createHttpClient({
  credentials: 'include',
});
const EmptyResponseSchema = z.undefined();

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

export type CreateDeveloperAppInput = {
  name: string;
  redirect_uris: string[];
  allowed_scopes: string[];
  allowed_audiences: string[];
  allowed_resources: string[];
};

export function createDeveloperApp(input: CreateDeveloperAppInput) {
  return developerHttpClient.post('/developer/apps', CreateDeveloperAppResponseSchema, input);
}

export function inspectDeveloperToken(token: string) {
  return developerHttpClient.post('/developer/tokens/inspect', InspectDeveloperTokenResponseSchema, {
    token,
  });
}

export type OAuthPlaygroundExchangeInput = {
  client_id: string;
  code: string;
  redirect_uri: string;
  code_verifier: string;
};

export function exchangeOAuthPlaygroundCode(input: OAuthPlaygroundExchangeInput) {
  return developerHttpClient.post(
    '/developer/oauth/playground/exchange',
    OAuthPlaygroundExchangeResponseSchema,
    input,
  );
}

export type DeveloperLogsFilter = {
  user_id?: string;
  client_id?: string;
  tenant_id?: string;
  event_type?: string;
};

export function listDeveloperLogs(filters: DeveloperLogsFilter, options?: { signal?: AbortSignal }) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    if (value) {
      params.set(key, value);
    }
  }
  const query = params.toString();

  return developerHttpClient
    .get(`/developer/logs${query ? `?${query}` : ''}`, DeveloperLogsResponseSchema, options)
    .then((response) => response.logs);
}

export type CreateDeveloperWebhookInput = {
  name: string;
  url: string;
  events: Array<'user.created' | 'login.failed' | 'session.revoked' | 'client.created'>;
};

export function createDeveloperWebhook(input: CreateDeveloperWebhookInput) {
  return developerHttpClient.post(
    '/developer/webhooks',
    CreateDeveloperWebhookEndpointResponseSchema,
    input,
  );
}

export function deleteDeveloperWebhook(endpointId: string) {
  return developerHttpClient
    .delete(`/developer/webhooks/${endpointId}`, EmptyResponseSchema)
    .then(() => undefined);
}

export function hasDeveloperPermission(
  me: DeveloperMe | undefined,
  permission: DeveloperPermission,
): boolean {
  return Boolean(me?.permissions.includes(permission));
}
