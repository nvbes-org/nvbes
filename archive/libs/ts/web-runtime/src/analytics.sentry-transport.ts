import type { BrowserAnalyticsTransportOptions } from './analytics.browser-transport';
import type { AnalyticsProperties } from './analytics.types';
import {
  getErrorReportingReplaysOnErrorSampleRate,
  getErrorReportingTracesSampleRate,
  scrubErrorReportingBreadcrumb,
  scrubErrorReportingEvent,
} from './error-reporting-privacy';

export function createSentryReporting(options: BrowserAnalyticsTransportOptions) {
  let sentry: typeof import('@sentry/browser') | null = null;
  let enabled = false;
  let pending = Promise.resolve();

  async function reconcile(): Promise<void> {
    if (!enabled) {
      const previous = sentry;
      sentry = null;
      await previous?.close(2_000);
      return;
    }
    const dsn = options.sentryDsn?.trim();
    if (sentry || !dsn || typeof window === 'undefined') return;
    const loaded = await import('@sentry/browser');
    // Consent may have been withdrawn during the asynchronous SDK import.
    if (!enabled) return;
    const production = options.environment === 'production';
    loaded.init({
      dsn,
      environment: options.environment,
      tracesSampleRate:
        options.sentryTracesSampleRate ?? getErrorReportingTracesSampleRate(production),
      replaysOnErrorSampleRate: getErrorReportingReplaysOnErrorSampleRate(production),
      sendDefaultPii: false,
      beforeBreadcrumb: scrubErrorReportingBreadcrumb,
      beforeSend: scrubErrorReportingEvent,
      initialScope: { tags: { app_name: options.appName } },
    });
    sentry = loaded;
  }

  return {
    setEnabled(this: void, value: boolean): Promise<void> {
      enabled = value;
      // Serialize SDK close/init so a late close cannot disable a newer instance.
      const operation = pending.then(reconcile);
      pending = operation.catch(() => undefined);
      return operation;
    },
    captureException(error: unknown, properties: AnalyticsProperties): void {
      if (enabled) sentry?.captureException(error, { extra: properties });
    },
  };
}
