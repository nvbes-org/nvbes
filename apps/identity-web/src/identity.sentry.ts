import * as Sentry from '@sentry/react';
import {
  type ClientErrorReportContext,
  createSentryFeedbackOptions,
  createSentryReplayPrivacyOptions,
  getSentryReplaysOnErrorSampleRate,
  getSentryTracesSampleRate,
  installBrowserSentrySmoke,
  isBrowserSentrySmokeEnabled,
  scrubSentryBreadcrumb,
  scrubSentryEvent,
} from '@nvbes/web-runtime';
import { isVendorAccepted } from './tracking-consent';

const APP_NAME = 'identity-web';
const RUNTIME = 'browser';

let initialized = false;

export function initSentry(): boolean {
  if (initialized) {
    installSentrySmoke(true, true);
    return true;
  }

  const dsn = import.meta.env.VITE_SENTRY_DSN;

  if (!isVendorAccepted('sentry')) {
    installSentrySmoke(false, Boolean(dsn));
    return false;
  }

  if (!dsn) {
    console.warn('Sentry DSN not found, skipping initialization');
    installSentrySmoke(false, false);
    return false;
  }

  Sentry.init({
    dsn,
    release: import.meta.env.VITE_SENTRY_RELEASE,
    environment: import.meta.env.VITE_SENTRY_ENVIRONMENT ?? import.meta.env.MODE,
    sendDefaultPii: false,
    integrations: [
      Sentry.browserTracingIntegration(),
      Sentry.replayIntegration(createSentryReplayPrivacyOptions()),
      Sentry.feedbackIntegration(createSentryFeedbackOptions(APP_NAME)),
    ],
    beforeSend: scrubSentryEvent,
    beforeSendTransaction: scrubSentryEvent,
    beforeBreadcrumb: scrubSentryBreadcrumb,
    // Performance Monitoring
    tracesSampleRate: getSentryTracesSampleRate(import.meta.env.PROD),
    tracePropagationTargets: ['localhost', /^https:\/\/api.nvbes.fr/],
    // Session Replay
    replaysSessionSampleRate: 0,
    replaysOnErrorSampleRate: getSentryReplaysOnErrorSampleRate(import.meta.env.PROD),
  });

  initialized = true;
  installSentrySmoke(true, true);
  return true;
}

export async function syncSentryConsent(): Promise<void> {
  const dsn = import.meta.env.VITE_SENTRY_DSN;
  const sentryAccepted = isVendorAccepted('sentry');

  if (sentryAccepted) {
    initSentry();
    return;
  }

  if (initialized) {
    await Sentry.close(2000);
    initialized = false;
  }

  installSentrySmoke(false, Boolean(dsn));
}

export function captureSentryException(error: Error, context: ClientErrorReportContext) {
  Sentry.captureException(error, context);
}

function installSentrySmoke(sentryInitialized: boolean, dsnConfigured: boolean) {
  installBrowserSentrySmoke({
    appName: APP_NAME,
    dsnConfigured,
    enabled: isBrowserSentrySmokeEnabled(
      import.meta.env.VITE_SENTRY_SMOKE_ENABLED,
      import.meta.env.DEV,
    ),
    environment: import.meta.env.VITE_SENTRY_ENVIRONMENT ?? import.meta.env.MODE,
    initialized: sentryInitialized,
    reporter: {
      captureMessage: (message, level) => Sentry.captureMessage(message, level),
      flush: (timeoutMs) => Sentry.flush(timeoutMs),
      withScope: (callback) => Sentry.withScope((scope) => callback(scope)),
    },
    runtime: RUNTIME,
  });
}
