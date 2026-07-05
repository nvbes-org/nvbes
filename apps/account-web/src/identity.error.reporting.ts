import * as Sentry from '@sentry/browser';
import {
  createErrorReportingReplayPrivacyOptions,
  getErrorReportingReplaysOnErrorSampleRate,
  getErrorReportingTracesSampleRate,
  scrubErrorReportingBreadcrumb,
  scrubErrorReportingEvent,
  type ClientErrorReportContext,
} from '@nvbes/web-runtime';
import { isCategoryAccepted, isVendorAccepted } from './tracking-consent';

const APP_NAME = 'account-web';
const CLOSE_TIMEOUT_MS = 2_000;

let initialized = false;
let enabled = false;

export function initErrorReporting(): boolean {
  const configured = isConfigured();
  if (configured && hasErrorReportingConsent()) {
    enableErrorReporting();
  }
  return configured;
}

export async function syncErrorReportingConsent(): Promise<void> {
  if (!isConfigured()) {
    return;
  }

  if (hasErrorReportingConsent()) {
    enableErrorReporting();
    return;
  }

  enabled = false;
  if (initialized) {
    await Sentry.close(CLOSE_TIMEOUT_MS);
    initialized = false;
  }
}

export function captureErrorReportingException(
  error: Error,
  context: ClientErrorReportContext,
): void {
  if (!enabled) {
    return;
  }

  Sentry.withScope((scope) => {
    scope.setTag('feature', context.tags.feature);
    scope.setTag('source', context.tags.source);
    scope.setContext('identity_web', {
      route_path: currentPath(),
    });
    Sentry.captureException(error);
  });
}

function enableErrorReporting(): void {
  if (initialized) {
    enabled = true;
    return;
  }

  const dsn = normalizedOptional(import.meta.env.VITE_SENTRY_DSN);
  if (!dsn || typeof window === 'undefined') {
    return;
  }

  const isProduction = import.meta.env.MODE === 'production';
  Sentry.init({
    dsn,
    environment: import.meta.env.MODE,
    release: releaseName(),
    tracesSampleRate:
      numberFromEnv(import.meta.env.VITE_SENTRY_TRACES_SAMPLE_RATE) ??
      getErrorReportingTracesSampleRate(isProduction),
    replaysSessionSampleRate: 0,
    replaysOnErrorSampleRate: getErrorReportingReplaysOnErrorSampleRate(isProduction),
    sendDefaultPii: false,
    attachStacktrace: true,
    integrations: [
      Sentry.browserTracingIntegration(),
      Sentry.replayIntegration(createErrorReportingReplayPrivacyOptions()),
    ],
    beforeBreadcrumb: scrubErrorReportingBreadcrumb,
    beforeSend: scrubErrorReportingEvent,
    initialScope: {
      tags: {
        app_name: APP_NAME,
        runtime: 'browser',
      },
    },
  });

  initialized = true;
  enabled = true;
}

function hasErrorReportingConsent(): boolean {
  return isVendorAccepted('sentry') || isCategoryAccepted('performance');
}

function isConfigured(): boolean {
  return (
    Boolean(normalizedOptional(import.meta.env.VITE_SENTRY_DSN)) && typeof window !== 'undefined'
  );
}

function releaseName(): string | undefined {
  return (
    normalizedOptional(import.meta.env.VITE_SENTRY_RELEASE) ??
    normalizedOptional(import.meta.env.VITE_NVBES_BUILD_ID)
  );
}

function currentPath(): string {
  if (typeof window === 'undefined') {
    return '/';
  }
  return window.location.pathname || '/';
}

function normalizedOptional(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function numberFromEnv(value: string | undefined): number | undefined {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return undefined;
  }
  return Math.min(Math.max(parsed, 0), 1);
}
