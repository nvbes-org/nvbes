const SMOKE_MESSAGE = 'nvbes browser sentry smoke test';
const SMOKE_TRANSACTION = 'observability.sentry_smoke';

export const SENTRY_SMOKE_GLOBAL = '__nvbesSentrySmoke';

type SmokeStatus = 'accepted' | 'skipped';
type SmokeSkipReason = 'sentry_not_initialized' | 'dsn_not_configured';

interface BrowserSentrySmokeScope {
  setTag(key: string, value: string | boolean): unknown;
  setTransactionName?(name?: string): unknown;
}

export interface BrowserSentrySmokeReporter {
  captureMessage(message: string, level: 'info'): string;
  flush(timeoutMs?: number): Promise<boolean>;
  withScope<T>(callback: (scope: BrowserSentrySmokeScope) => T): T;
}

export interface BrowserSentrySmokeOptions {
  appName: string;
  dsnConfigured: boolean;
  enabled: boolean;
  environment: string;
  initialized: boolean;
  reporter: BrowserSentrySmokeReporter;
  runtime: 'browser';
}

export interface BrowserSentrySmokeResult {
  appName: string;
  dsnConfigured: boolean;
  environment: string;
  eventId: string | null;
  flushed: boolean;
  initialized: boolean;
  reason?: SmokeSkipReason;
  runtime: 'browser';
  status: SmokeStatus;
}

declare global {
  interface Window {
    __nvbesSentrySmoke?: () => Promise<BrowserSentrySmokeResult>;
  }
}

export function isBrowserSentrySmokeEnabled(value: string | undefined, isDevelopment: boolean) {
  return isDevelopment || value === '1' || value === 'true';
}

export function installBrowserSentrySmoke(options: BrowserSentrySmokeOptions): boolean {
  if (typeof window === 'undefined' || !options.enabled) {
    return false;
  }

  window[SENTRY_SMOKE_GLOBAL] = () => captureBrowserSentrySmoke(options);
  return true;
}

async function captureBrowserSentrySmoke(
  options: BrowserSentrySmokeOptions,
): Promise<BrowserSentrySmokeResult> {
  if (!options.dsnConfigured) {
    return skippedSmokeResult(options, 'dsn_not_configured');
  }

  if (!options.initialized) {
    return skippedSmokeResult(options, 'sentry_not_initialized');
  }

  const eventId = options.reporter.withScope((scope) => {
    scope.setTransactionName?.(SMOKE_TRANSACTION);
    scope.setTag('app', options.appName);
    scope.setTag('environment', options.environment);
    scope.setTag('runtime', options.runtime);
    scope.setTag('smoke_test', 'sentry');

    return options.reporter.captureMessage(SMOKE_MESSAGE, 'info');
  });
  const flushed = await options.reporter.flush(2000).catch(() => false);

  return {
    appName: options.appName,
    dsnConfigured: options.dsnConfigured,
    environment: options.environment,
    eventId,
    flushed,
    initialized: true,
    runtime: options.runtime,
    status: 'accepted',
  };
}

function skippedSmokeResult(
  options: BrowserSentrySmokeOptions,
  reason: SmokeSkipReason,
): BrowserSentrySmokeResult {
  return {
    appName: options.appName,
    dsnConfigured: options.dsnConfigured,
    environment: options.environment,
    eventId: null,
    flushed: false,
    initialized: options.initialized,
    reason,
    runtime: options.runtime,
    status: 'skipped',
  };
}
