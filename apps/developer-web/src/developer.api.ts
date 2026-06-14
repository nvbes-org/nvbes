import { identityHttpClient } from './identity.http';
import {
  DeveloperContextSchema,
  DeveloperOverviewSchema,
  type DeveloperContext,
  type DeveloperOverview,
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
