import { identityHttpClient } from './identity.http';
import {
  DeveloperContextSchema,
  DeveloperLogsSchema,
  DeveloperMarketplaceAppsSchema,
  DeveloperOAuthClientsSchema,
  DeveloperOverviewSchema,
  DeveloperSecretVersionsSchema,
  DeveloperServiceAccountsSchema,
  DeveloperScopeRegistrySchema,
  DeveloperWebhookEndpointsSchema,
  RotateDeveloperSecretSchema,
  type DeveloperContext,
  type DeveloperLogEntry,
  type DeveloperMarketplaceApp,
  type DeveloperOAuthClient,
  type DeveloperOverview,
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
