import { QueryClient, keepPreviousData } from '@tanstack/react-query';
import { DtoValidationError, HttpError } from '@nvbes/http-client';

export type ClientErrorKind = 'api' | 'dto' | 'unexpected';

export class ClientRuntimeError extends Error {
  readonly kind: ClientErrorKind;
  readonly status?: number;
  readonly body?: unknown;
  readonly cause: unknown;
  readonly requestId?: string;

  constructor(
    kind: ClientErrorKind,
    message: string,
    cause: unknown,
    status?: number,
    body?: unknown,
    requestId?: string,
  ) {
    super(message);
    this.name = 'ClientRuntimeError';
    this.kind = kind;
    this.status = status;
    this.body = body;
    this.cause = cause;
    this.requestId = requestId;
  }
}

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        gcTime: 30 * 60 * 1000,
        placeholderData: keepPreviousData,
        retry: (failureCount, error) => {
          const normalized = normalizeClientError(error);
          if (normalized.kind === 'api' && normalized.status && normalized.status < 500) {
            return false;
          }
          return failureCount < 2;
        },
        staleTime: 30 * 1000,
        refetchOnWindowFocus: false,
        throwOnError: false,
      },
      mutations: {
        retry: false,
      },
    },
  });
}

export function normalizeClientError(error: unknown): ClientRuntimeError {
  if (error instanceof ClientRuntimeError) {
    return error;
  }

  if (error instanceof HttpError) {
    return new ClientRuntimeError(
      'api',
      error.message,
      error,
      error.status,
      error.body,
      error.requestId,
    );
  }

  if (error instanceof DtoValidationError) {
    return new ClientRuntimeError('dto', error.message, error);
  }

  if (error instanceof Error) {
    return new ClientRuntimeError('unexpected', error.message, error);
  }

  return new ClientRuntimeError('unexpected', 'Unexpected client runtime error', error);
}

export { ErrorBoundary } from './ErrorBoundary';
export type { ErrorBoundaryProps } from './ErrorBoundary';
export { RouterErrorFallback } from './RouterErrorFallback';
export {
  AuthErrorBoundary,
  AuthErrorFallback,
  BillingErrorBoundary,
  BillingErrorFallback,
  EditorErrorBoundary,
  EditorErrorFallback,
  UploadErrorBoundary,
  UploadErrorFallback,
} from './feature-fallbacks';
export { configureErrorReporting, reportClientError } from './report-error';
export type { ClientErrorReportContext, ClientErrorReporter } from './report-error';
export {
  createSentryReplayPrivacyOptions,
  getSentryReplaysOnErrorSampleRate,
  getSentryTracesSampleRate,
  sanitizeUrlString,
  scrubReplayRecordingEvent,
  scrubSentryBreadcrumb,
  scrubSentryEvent,
} from './sentry-privacy';
export { createSentryFeedbackOptions } from './sentry-feedback';
export {
  installBrowserSentrySmoke,
  isBrowserSentrySmokeEnabled,
  SENTRY_SMOKE_GLOBAL,
} from './sentry-smoke';
export type {
  BrowserSentrySmokeOptions,
  BrowserSentrySmokeReporter,
  BrowserSentrySmokeResult,
} from './sentry-smoke';
export {
  ACCEPT_ALL_CONSENT,
  CATEGORY_POSTHOG_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  createTrackingConsentApi,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  POSTHOG_PURPOSE_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
} from './tracking-consent';
export type {
  CookieConsentState,
  TrackingConsentClient,
  TrackingConsentStoredValue,
} from './tracking-consent';
export {
  ANALYTICS_POSTHOG_PURPOSES,
  deriveConsentState,
  hasAnyPostHogPurpose,
  toggleConsentCategory,
  toggleConsentPostHogPurpose,
  toggleConsentVendor,
  type PostHogPurpose,
} from './tracking-consent.editor';
export { SharedTrackingConsentBanner } from './TrackingConsentBanner';
export { TrackingConsentToggle } from './TrackingConsentToggle';

export function clientErrorMessage(error: unknown, fallback = 'Une erreur est survenue.'): string {
  if (error instanceof ClientRuntimeError) {
    const base = error.message || fallback;
    return error.requestId ? `${base} (ref: ${error.requestId})` : base;
  }

  if (error instanceof Error) {
    return error.message || fallback;
  }

  return fallback;
}

export {
  clearAppBadge,
  getInstallPrompt,
  getSwReady,
  getWebAppDisplayMode,
  getWebPushSupport,
  isInstalledWebApp,
  migrateServiceWorkers,
  onBackgroundFetchEvent,
  onInstallReady,
  queueMutation,
  registerBackgroundSync,
  registerPeriodicSync,
  registerServiceWorker,
  requestWebPushPermission,
  setAppBadge,
  sendNetworkQualityToSw,
  sendToSw,
  startBackgroundFetch,
  supportsAppBadging,
} from './service-worker';
export type { WebAppDisplayMode, WebPushSupport } from './service-worker';
export {
  broadcast,
  broadcastLogout,
  broadcastTokenRefreshed,
  broadcastWorkspaceChange,
  onBroadcast,
} from './broadcast';
export { acquireTokenRefreshLock, withTokenRefreshLock } from './token-lock';
export { cryptoWorker } from './worker.crypto.client';
export { createWorker, defineWorker } from './worker';
export { useNetworkQuality } from './use-network-quality';
export type { NetworkQuality, EffectiveType } from './use-network-quality';
