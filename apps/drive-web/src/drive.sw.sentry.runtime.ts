/// <reference types="vite/client" />

import * as Sentry from '@sentry/browser';
import { normalizeServiceWorkerError, scrubUnknownValue } from './drive.sw.sentry.scrub';

let initialized = false;
let consentGranted = false;

export function setDriveServiceWorkerSentryConsent(accepted: boolean): boolean {
  consentGranted = accepted;
  if (!accepted) {
    return false;
  }

  return initDriveServiceWorkerSentry();
}

function initDriveServiceWorkerSentry(): boolean {
  if (initialized) {
    return true;
  }

  const dsn = import.meta.env.VITE_SENTRY_DSN;
  if (!dsn) {
    return false;
  }

  Sentry.init({
    dsn,
    release: import.meta.env.VITE_SENTRY_RELEASE,
    environment: import.meta.env.VITE_SENTRY_ENVIRONMENT ?? import.meta.env.MODE,
    sendDefaultPii: false,
    defaultIntegrations: false,
    beforeSend: (event) => scrubUnknownValue(event) as typeof event,
  });
  Sentry.setTag('runtime', 'service-worker');
  Sentry.setTag('app', 'drive-web');

  initialized = true;
  return true;
}

export function captureDriveServiceWorkerException(
  error: unknown,
  operation: string,
  context?: Record<string, unknown>,
) {
  if (!initialized || !consentGranted) {
    return;
  }

  Sentry.withScope((scope) => {
    scope.setTag('operation', operation);
    if (context) {
      scope.setContext('service_worker', scrubUnknownValue(context) as Record<string, unknown>);
    }
    Sentry.captureException(normalizeServiceWorkerError(error));
  });
}
