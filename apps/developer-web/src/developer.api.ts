import { identityHttpClient } from './identity.http';
import {
  DeveloperContextSchema,
  DeveloperMarketplaceAppsSchema,
  DeveloperOAuthClientsSchema,
  DeveloperOverviewSchema,
  DeveloperScopeRegistrySchema,
  type DeveloperContext,
  type DeveloperMarketplaceApp,
  type DeveloperOAuthClient,
  type DeveloperOverview,
  type DeveloperScopeRegistryEntry,
} from './developer.schemas';

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
