import { createHttpClient } from '@nvbes/http-client';
import { z } from 'zod';
import { identityApiBaseUrl, identityHttpClient, identityVerifiedFetch } from './identity.http';
import {
  CreateDeveloperAppResponseSchema,
  CreateDeveloperWebhookEndpointResponseSchema,
  DebugDeveloperTokenSchema,
  DeveloperAppsResponseSchema,
  DeveloperConsentScreenSchema,
  DeveloperContextSchema,
  DeveloperHealthChecksSchema,
  DeveloperLogsSchema,
  DeveloperMarketplaceAppSchema,
  DeveloperMarketplaceAppsSchema,
  DeveloperMeSchema,
  DeveloperOAuthClientsSchema,
  DeveloperOverviewSchema,
  DeveloperPortalLogsResponseSchema,
  DeveloperSandboxSchema,
  DeveloperSandboxTenantSchema,
  DeveloperSecretVersionsSchema,
  DeveloperServiceAccountsSchema,
  DeveloperScopeRegistrySchema,
  DeveloperWebhookDeliveriesSchema,
  DeveloperWebhookDeliverySchema,
  DeveloperWebhookEndpointsSchema,
  DeveloperWebhooksResponseSchema,
  InspectDeveloperTokenResponseSchema,
  OAuthPlaygroundExchangeResponseSchema,
  RotateDeveloperSecretSchema,
  type DebugDeveloperToken,
  type DeveloperConsentScreen,
  type DeveloperContext,
  type DeveloperHealthCheck,
  type DeveloperLogEntry,
  type DeveloperMarketplaceApp,
  type DeveloperMe,
  DeveloperScopeRegistryEntrySchema,
  type DeveloperOAuthClient,
  type DeveloperOverview,
  type DeveloperPermission,
  type DeveloperSandboxTenant,
  type DeveloperSandboxDataProfile,
  type DeveloperSecretVersion,
  type DeveloperServiceAccount,
  type DeveloperScopeRegistryEntry,
  type DeveloperWebhookEndpoint,
  type DeveloperWebhookDelivery,
  type PortalDeveloperLogEntry,
  type PortalDeveloperWebhookEndpoint,
  type RotateDeveloperSecret,
  type CreateScopeInput,
  type UpdateScopeInput,
} from './developer.schemas';
import type { SecretRotationForm } from './pages/SecretsPage.helpers';
import { buildSecretRotationPayload } from './pages/SecretsPage.helpers';

const developerHttpClient = createHttpClient({
  baseUrl: identityApiBaseUrl,
  credentials: 'include',
  fetchImpl: identityVerifiedFetch,
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

export function listDeveloperWebhooks(options?: {
  signal?: AbortSignal;
}): Promise<PortalDeveloperWebhookEndpoint[]> {
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
  return developerHttpClient.post(
    '/developer/tokens/inspect',
    InspectDeveloperTokenResponseSchema,
    {
      token,
    },
  );
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

export function listDeveloperLogs(
  filters: DeveloperLogsFilter,
  options?: { signal?: AbortSignal },
): Promise<PortalDeveloperLogEntry[]> {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    if (value) {
      params.set(key, value);
    }
  }
  const query = params.toString();

  return developerHttpClient
    .get(`/developer/logs${query ? `?${query}` : ''}`, DeveloperPortalLogsResponseSchema, options)
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

export function getDeveloperContext(signal?: AbortSignal): Promise<DeveloperContext> {
  return identityHttpClient.get('/developer/console/context', DeveloperContextSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
}

export function getDeveloperOverview(signal?: AbortSignal): Promise<DeveloperOverview> {
  return identityHttpClient.get('/developer/console/overview', DeveloperOverviewSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
}

export async function listDeveloperOAuthClients(
  signal?: AbortSignal,
): Promise<DeveloperOAuthClient[]> {
  const response = await identityHttpClient.get(
    '/developer/console/oauth-clients',
    DeveloperOAuthClientsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.oauth_clients;
}

export async function listDeveloperMarketplaceApps(
  signal?: AbortSignal,
): Promise<DeveloperMarketplaceApp[]> {
  const response = await identityHttpClient.get(
    '/developer/console/marketplace/apps',
    DeveloperMarketplaceAppsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.apps;
}

export function submitDeveloperMarketplaceApp(clientId: string): Promise<DeveloperMarketplaceApp> {
  return identityHttpClient.post(
    `/developer/console/marketplace/apps/${encodeURIComponent(clientId)}/submit`,
    DeveloperMarketplaceAppSchema,
    {},
  );
}

export function reviewDeveloperMarketplaceApp(
  clientId: string,
  status: string,
  reviewReason?: string,
): Promise<DeveloperMarketplaceApp> {
  return identityHttpClient.post(
    `/developer/console/marketplace/apps/${encodeURIComponent(clientId)}/review`,
    DeveloperMarketplaceAppSchema,
    {
      status,
      review_reason: reviewReason || null,
    },
  );
}

export async function listDeveloperScopes(
  signal?: AbortSignal,
): Promise<DeveloperScopeRegistryEntry[]> {
  const response = await identityHttpClient.get(
    '/developer/console/scopes',
    DeveloperScopeRegistrySchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.scopes;
}

export function getDeveloperConsentScreen(
  clientId: string,
  signal?: AbortSignal,
): Promise<DeveloperConsentScreen> {
  return identityHttpClient.get(
    `/developer/console/oauth-clients/${encodeURIComponent(clientId)}/consent-screen`,
    DeveloperConsentScreenSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
}

export function upsertDeveloperConsentScreen(
  clientId: string,
  form: DeveloperConsentScreenForm,
): Promise<DeveloperConsentScreen> {
  return identityHttpClient.request(
    `/developer/console/oauth-clients/${encodeURIComponent(clientId)}/consent-screen`,
    DeveloperConsentScreenSchema,
    {
      method: 'PUT',
      body: buildConsentScreenPayload(form),
    },
  );
}

export async function listDeveloperServiceAccounts(
  signal?: AbortSignal,
): Promise<DeveloperServiceAccount[]> {
  const response = await identityHttpClient.get(
    '/developer/console/service-accounts',
    DeveloperServiceAccountsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.service_accounts;
}

export type DeveloperConsentScreenForm = {
  productName: string;
  description: string;
  logoUrl: string;
  supportUrl: string;
  privacyUrl: string;
  termsUrl: string;
  brandColor: string;
  customCss: string;
  helpText: string;
};

function buildConsentScreenPayload(form: DeveloperConsentScreenForm) {
  return {
    product_name: form.productName.trim(),
    description: form.description.trim(),
    logo_url: nullableTrim(form.logoUrl),
    support_url: nullableTrim(form.supportUrl),
    privacy_url: nullableTrim(form.privacyUrl),
    terms_url: nullableTrim(form.termsUrl),
    brand_color: nullableTrim(form.brandColor),
    custom_css: nullableTrim(form.customCss),
    help_text: nullableTrim(form.helpText),
  };
}

function nullableTrim(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export async function listDeveloperSecretVersions(
  clientId: string,
  signal?: AbortSignal,
): Promise<DeveloperSecretVersion[]> {
  const response = await identityHttpClient.get(
    `/developer/console/oauth-clients/${encodeURIComponent(clientId)}/secrets`,
    DeveloperSecretVersionsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.secret_versions;
}

export function rotateDeveloperSecret(
  clientId: string,
  form: SecretRotationForm,
): Promise<RotateDeveloperSecret> {
  return identityHttpClient.post(
    `/developer/console/oauth-clients/${encodeURIComponent(clientId)}/secrets/rotation`,
    RotateDeveloperSecretSchema,
    buildSecretRotationPayload(form),
  );
}

export async function revokeDeveloperSecretVersion(
  clientId: string,
  versionId: string,
): Promise<DeveloperSecretVersion[]> {
  const response = await identityHttpClient.post(
    `/developer/console/oauth-clients/${encodeURIComponent(clientId)}/secrets/${encodeURIComponent(versionId)}/revoke`,
    DeveloperSecretVersionsSchema,
    {},
  );
  return response.secret_versions;
}

export async function listDeveloperConsoleWebhooks(
  signal?: AbortSignal,
): Promise<DeveloperWebhookEndpoint[]> {
  const response = await identityHttpClient.get(
    '/developer/console/webhooks',
    DeveloperWebhookEndpointsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.webhooks;
}

export async function listDeveloperConsoleLogs(signal?: AbortSignal): Promise<DeveloperLogEntry[]> {
  const response = await identityHttpClient.get('/developer/console/logs', DeveloperLogsSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
  return response.logs;
}

export function debugDeveloperToken(accessToken: string): Promise<DebugDeveloperToken> {
  return identityHttpClient.post('/developer/console/tokens/debug', DebugDeveloperTokenSchema, {
    access_token: accessToken,
  });
}

export async function getDeveloperSandbox(
  signal?: AbortSignal,
): Promise<DeveloperSandboxTenant | null> {
  const response = await identityHttpClient.get(
    '/developer/console/sandbox',
    DeveloperSandboxSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.sandbox;
}

export function upsertDeveloperSandbox(
  dataProfile: DeveloperSandboxDataProfile,
): Promise<DeveloperSandboxTenant> {
  return identityHttpClient.request('/developer/console/sandbox', DeveloperSandboxTenantSchema, {
    method: 'PUT',
    body: {
      data_profile: dataProfile,
    },
  });
}

export function resetDeveloperSandbox(): Promise<DeveloperSandboxTenant> {
  return identityHttpClient.post(
    '/developer/console/sandbox/reset',
    DeveloperSandboxTenantSchema,
    {},
  );
}

export async function listDeveloperHealthChecks(
  signal?: AbortSignal,
): Promise<DeveloperHealthCheck[]> {
  const response = await identityHttpClient.get(
    '/developer/console/health-checks',
    DeveloperHealthChecksSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.checks;
}

export async function runDeveloperHealthChecks(): Promise<DeveloperHealthCheck[]> {
  const response = await identityHttpClient.post(
    '/developer/console/health-checks',
    DeveloperHealthChecksSchema,
    {},
  );
  return response.checks;
}

export function createDeveloperScope(
  input: CreateScopeInput,
): Promise<DeveloperScopeRegistryEntry> {
  return identityHttpClient.post(
    '/developer/console/scopes',
    DeveloperScopeRegistryEntrySchema,
    input,
  );
}

export function updateDeveloperScope(
  scopeKey: string,
  input: UpdateScopeInput,
): Promise<DeveloperScopeRegistryEntry> {
  return identityHttpClient.request(
    `/developer/console/scopes/${encodeURIComponent(scopeKey)}`,
    DeveloperScopeRegistryEntrySchema,
    {
      method: 'PUT',
      body: input,
    },
  );
}

export function deleteDeveloperScope(scopeKey: string): Promise<void> {
  return identityHttpClient.delete(
    `/developer/console/scopes/${encodeURIComponent(scopeKey)}`,
    EmptyResponseSchema,
  );
}

export async function listDeveloperConsoleWebhookDeliveries(
  endpointId: string,
  signal?: AbortSignal,
): Promise<DeveloperWebhookDelivery[]> {
  const response = await identityHttpClient.get(
    `/developer/console/webhooks/${encodeURIComponent(endpointId)}/deliveries`,
    DeveloperWebhookDeliveriesSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.deliveries;
}

export async function replayDeveloperConsoleWebhookDelivery(
  deliveryId: string,
): Promise<DeveloperWebhookDelivery> {
  return identityHttpClient.post(
    `/developer/console/webhooks/deliveries/${encodeURIComponent(deliveryId)}/replay`,
    DeveloperWebhookDeliverySchema,
    {},
  );
}

function withTimeoutSignal(signal: AbortSignal | undefined, timeoutMs: number): AbortSignal {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), timeoutMs);

  function abort(): void {
    window.clearTimeout(timeout);
    controller.abort();
  }

  if (signal?.aborted) {
    abort();
    return controller.signal;
  }

  signal?.addEventListener('abort', abort, { once: true });
  controller.signal.addEventListener('abort', () => window.clearTimeout(timeout), { once: true });

  return controller.signal;
}
