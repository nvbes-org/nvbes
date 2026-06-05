import * as Sentry from '@sentry/react';
import {
  type ClientErrorReportContext,
  createSentryFeedbackOptions,
  createSentryReplayPrivacyOptions,
  getSentryReplaysOnErrorSampleRate,
  getSentryTracesSampleRate,
  scrubSentryBreadcrumb,
  scrubSentryEvent,
} from '@nvbes/web-runtime';
import { isVendorAccepted } from './tracking-consent';

let initialized = false;

export function initSentry(): boolean {
  if (initialized) {
    return true;
  }

  if (!isVendorAccepted('sentry')) {
    return false;
  }

  const dsn = import.meta.env.VITE_SENTRY_DSN;
  if (!dsn) {
    console.warn('Sentry DSN not found, skipping initialization');
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
      Sentry.feedbackIntegration(createSentryFeedbackOptions('drive-web')),
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
  return true;
}

export function captureSentryException(error: Error, context: ClientErrorReportContext) {
  Sentry.captureException(error, context);
}
