import { identityHttpClient } from './identity.http';
import {
  DebugDeveloperTokenSchema,
  DeveloperConsentScreenSchema,
  DeveloperContextSchema,
  DeveloperHealthChecksSchema,
  DeveloperLogsSchema,
  DeveloperMarketplaceAppsSchema,
  DeveloperOAuthClientsSchema,
  DeveloperOverviewSchema,
  DeveloperSandboxSchema,
  DeveloperSandboxTenantSchema,
  DeveloperSecretVersionsSchema,
  DeveloperServiceAccountsSchema,
  DeveloperScopeRegistrySchema,
  DeveloperWebhookEndpointsSchema,
  RotateDeveloperSecretSchema,
  type DebugDeveloperToken,
  type DeveloperConsentScreen,
  type DeveloperContext,
  type DeveloperHealthCheck,
  type DeveloperLogEntry,
  type DeveloperMarketplaceApp,
  type DeveloperOAuthClient,
  type DeveloperOverview,
  type DeveloperSandboxTenant,
  type DeveloperSecretVersion,
  type DeveloperServiceAccount,
  type DeveloperScopeRegistryEntry,
  type DeveloperWebhookEndpoint,
  type RotateDeveloperSecret,
} from './developer.schemas';
import type { SecretRotationForm } from './pages/SecretsPage.helpers';
import { buildSecretRotationPayload } from './pages/SecretsPage.helpers';

export function getDeveloperContext(signal?: AbortSignal): Promise<DeveloperContext> {
  return identityHttpClient.get('/developer/context', DeveloperContextSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
}

export function getDeveloperOverview(signal?: AbortSignal): Promise<DeveloperOverview> {
  return identityHttpClient.get('/developer/overview', DeveloperOverviewSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
}

export async function listDeveloperOAuthClients(
  signal?: AbortSignal,
): Promise<DeveloperOAuthClient[]> {
  const response = await identityHttpClient.get(
    '/developer/oauth-clients',
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
    '/developer/marketplace/apps',
    DeveloperMarketplaceAppsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.apps;
}

export async function listDeveloperScopes(
  signal?: AbortSignal,
): Promise<DeveloperScopeRegistryEntry[]> {
  const response = await identityHttpClient.get('/developer/scopes', DeveloperScopeRegistrySchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
  return response.scopes;
}

export function getDeveloperConsentScreen(
  clientId: string,
  signal?: AbortSignal,
): Promise<DeveloperConsentScreen> {
  return identityHttpClient.get(
    `/developer/oauth-clients/${encodeURIComponent(clientId)}/consent-screen`,
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
    `/developer/oauth-clients/${encodeURIComponent(clientId)}/consent-screen`,
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
    '/developer/service-accounts',
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
};

function buildConsentScreenPayload(form: DeveloperConsentScreenForm) {
  return {
    product_name: form.productName.trim(),
    description: form.description.trim(),
    logo_url: nullableTrim(form.logoUrl),
    support_url: nullableTrim(form.supportUrl),
    privacy_url: nullableTrim(form.privacyUrl),
    terms_url: nullableTrim(form.termsUrl),
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
    `/developer/oauth-clients/${encodeURIComponent(clientId)}/secrets`,
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
    `/developer/oauth-clients/${encodeURIComponent(clientId)}/secrets/rotation`,
    RotateDeveloperSecretSchema,
    buildSecretRotationPayload(form),
  );
}

export async function listDeveloperWebhooks(
  signal?: AbortSignal,
): Promise<DeveloperWebhookEndpoint[]> {
  const response = await identityHttpClient.get(
    '/developer/webhooks',
    DeveloperWebhookEndpointsSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.webhooks;
}

export async function listDeveloperLogs(signal?: AbortSignal): Promise<DeveloperLogEntry[]> {
  const response = await identityHttpClient.get('/developer/logs', DeveloperLogsSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
  return response.logs;
}

export function debugDeveloperToken(accessToken: string): Promise<DebugDeveloperToken> {
  return identityHttpClient.post('/developer/tokens/debug', DebugDeveloperTokenSchema, {
    access_token: accessToken,
  });
}

export async function getDeveloperSandbox(
  signal?: AbortSignal,
): Promise<DeveloperSandboxTenant | null> {
  const response = await identityHttpClient.get('/developer/sandbox', DeveloperSandboxSchema, {
    signal: withTimeoutSignal(signal, 4_000),
  });
  return response.sandbox;
}

export function upsertDeveloperSandbox(dataProfile: string): Promise<DeveloperSandboxTenant> {
  return identityHttpClient.request('/developer/sandbox', DeveloperSandboxTenantSchema, {
    method: 'PUT',
    body: {
      data_profile: dataProfile,
    },
  });
}

export function resetDeveloperSandbox(): Promise<DeveloperSandboxTenant> {
  return identityHttpClient.post('/developer/sandbox/reset', DeveloperSandboxTenantSchema, {});
}

export async function listDeveloperHealthChecks(
  signal?: AbortSignal,
): Promise<DeveloperHealthCheck[]> {
  const response = await identityHttpClient.get(
    '/developer/health-checks',
    DeveloperHealthChecksSchema,
    {
      signal: withTimeoutSignal(signal, 4_000),
    },
  );
  return response.checks;
}

export async function runDeveloperHealthChecks(): Promise<DeveloperHealthCheck[]> {
  const response = await identityHttpClient.post(
    '/developer/health-checks',
    DeveloperHealthChecksSchema,
    {},
  );
  return response.checks;
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
