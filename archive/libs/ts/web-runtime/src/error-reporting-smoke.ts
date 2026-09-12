const SMOKE_MESSAGE = 'nvbes browser errorReporting smoke test';
const SMOKE_TRANSACTION = 'observability.errorReporting_smoke';

export const ERROR_REPORTING_SMOKE_GLOBAL = '__nvbesErrorReportingSmoke';

type SmokeStatus = 'accepted' | 'skipped';
type SmokeSkipReason = 'errorReporting_not_initialized' | 'dsn_not_configured';

interface BrowserErrorReportingSmokeScope {
  setTag(key: string, value: string | boolean): unknown;
  setTransactionName?(name?: string): unknown;
}

export interface BrowserErrorReportingSmokeReporter {
  captureMessage(message: string, level: 'info'): string;
  flush(timeoutMs?: number): Promise<boolean>;
  withScope<T>(callback: (scope: BrowserErrorReportingSmokeScope) => T): T;
}

export interface BrowserErrorReportingSmokeOptions {
  appName: string;
  dsnConfigured: boolean;
  enabled: boolean;
  environment: string;
  initialized: boolean;
  reporter: BrowserErrorReportingSmokeReporter;
  runtime: 'browser';
}

export interface BrowserErrorReportingSmokeResult {
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
    __nvbesErrorReportingSmoke?: () => Promise<BrowserErrorReportingSmokeResult>;
  }
}

export function isBrowserErrorReportingSmokeEnabled(
  value: string | undefined,
  isDevelopment: boolean,
) {
  return isDevelopment || value === '1' || value === 'true';
}

export function installBrowserErrorReportingSmoke(
  options: BrowserErrorReportingSmokeOptions,
): boolean {
  if (typeof window === 'undefined' || !options.enabled) {
    return false;
  }

  window[ERROR_REPORTING_SMOKE_GLOBAL] = () => captureBrowserErrorReportingSmoke(options);
  return true;
}

async function captureBrowserErrorReportingSmoke(
  options: BrowserErrorReportingSmokeOptions,
): Promise<BrowserErrorReportingSmokeResult> {
  if (!options.dsnConfigured) {
    return skippedSmokeResult(options, 'dsn_not_configured');
  }

  if (!options.initialized) {
    return skippedSmokeResult(options, 'errorReporting_not_initialized');
  }

  const eventId = options.reporter.withScope((scope) => {
    scope.setTransactionName?.(SMOKE_TRANSACTION);
    scope.setTag('app', options.appName);
    scope.setTag('environment', options.environment);
    scope.setTag('runtime', options.runtime);
    scope.setTag('smoke_test', 'errorReporting');

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
  options: BrowserErrorReportingSmokeOptions,
  reason: SmokeSkipReason,
): BrowserErrorReportingSmokeResult {
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
