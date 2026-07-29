import { QueryClient, keepPreviousData } from '@tanstack/react-query';
import { DtoValidationError, HttpError } from '@nvbes/http-client';
import { isSessionStaleError } from './session-stale';

export * from './csp';

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
          if (isSessionStaleError(error)) {
            return false;
          }
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
export { ErrorWithRetry, FeatureBoundary } from './error-with-retry';
export type { ErrorWithRetryProps, FeatureBoundaryProps } from './error-with-retry';
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
  createErrorReportingReplayPrivacyOptions,
  getErrorReportingReplaysOnErrorSampleRate,
  getErrorReportingTracesSampleRate,
  sanitizeUrlString,
  scrubReplayRecordingEvent,
  scrubErrorReportingBreadcrumb,
  scrubErrorReportingEvent,
} from './error-reporting-privacy';
export { createErrorReportingFeedbackOptions } from './error-reporting-feedback';
export {
  installBrowserErrorReportingSmoke,
  isBrowserErrorReportingSmokeEnabled,
  ERROR_REPORTING_SMOKE_GLOBAL,
} from './error-reporting-smoke';
export type {
  BrowserErrorReportingSmokeOptions,
  BrowserErrorReportingSmokeReporter,
  BrowserErrorReportingSmokeResult,
} from './error-reporting-smoke';
export {
  ACCEPT_ALL_CONSENT,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  createTrackingConsentApi,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
} from './tracking-consent';
export type {
  CookieConsentState,
  TrackingConsentClient,
  TrackingConsentStoredValue,
} from './tracking-consent';
export {
  deriveConsentState,
  hasAnyAnalyticsPurpose,
  OPTIONAL_ANALYTICS_PURPOSES,
  revokeTrackingConsentType,
  toggleConsentCategory,
  toggleConsentAnalyticsPurpose,
  toggleConsentVendor,
  type AnalyticsPurpose,
} from './tracking-consent.editor';
export { SharedTrackingConsentBanner } from './TrackingConsentBanner';
export type { TrackingConsentToggleComponent } from './TrackingConsentBanner';
export { TrackingConsentToggle } from './TrackingConsentToggle';
export type { TrackingConsentToggleProps } from './TrackingConsentToggle';
export { createSafeStorage, getSafeLocalStorage, getSafeSessionStorage } from './safe-storage';
export type { SafeStorage, SafeStorageKind } from './safe-storage';
export {
  createVerifiedFetch,
  verifiedFetch,
  verifiedFetchJson,
  VerifiedFetchError,
} from './verified-fetch';
export type { VerifiedFetchInit, VerifiedFetchInput, VerifiedFetchOptions } from './verified-fetch';
export { sanitizeHtml, safeHtmlToString, trustSafeHtml, VerifiedHtml } from './safe-html';
export type { SafeHtml } from './safe-html';
export {
  createTrustedWorkerScriptUrl,
  installDefaultTrustedTypesPolicy,
  type NvbesTrustedScriptUrl,
} from './trusted-types';
export { sanitizeStyleElementCss, safeStyleElementCssToString } from './safe-css';
export type { SafeStyleElementCss } from './safe-css';
export { sanitizeUrlForAttribute, safeUrlToString } from './safe-url';
export type { SafeUrl, SafeUrlOptions } from './safe-url';
export {
  findInvisibleUnicodeCharacters,
  hasInvisibleUnicodeCharacters,
  InvisibleUnicodeWarning,
  revealInvisibleUnicodeCharacters,
} from './unicode-invisible';
export type { InvisibleUnicodeMatch } from './unicode-invisible';
export { formatAbsoluteDateTime, formatRelativeDateTime, RelativeTime } from './relative-time';
export type { RelativeTimeInput, RelativeTimeProps } from './relative-time';
export {
  detectVersionMismatch,
  inspectVersionMismatch,
  recordVersionMismatchPrompt,
} from './version-mismatch';
export type {
  VersionMismatchDetectorOptions,
  VersionMismatchInput,
  VersionMismatchResult,
} from './version-mismatch';
export { VersionMismatchBanner } from './version-mismatch-banner';
export type { VersionMismatchBannerProps } from './version-mismatch-banner';
export {
  getBrowserVisibilityState,
  useBrowserVisibilityState,
  useVisibilityAwareInterval,
} from './live-updates';
export type { BrowserVisibilityState } from './live-updates';
export { LiveRegionMessage, LiveRegionProvider, useLiveRegion } from './live-region';
export type { LiveRegionApi } from './live-region';
export { commandSearchTokenValue, parseCommandSearch } from './command-search';
export type { CommandSearchToken, ParsedCommandSearch } from './command-search';
export { detectSessionStale, isSessionStaleError } from './session-stale';
export type { SessionStaleReason, SessionStaleResult } from './session-stale';

export function clientErrorMessage(error: unknown, fallback = 'Une erreur est survenue.'): string {
  const normalizedError = normalizeClientError(error);
  const base = clientRuntimeErrorMessage(normalizedError, fallback);
  return normalizedError.requestId ? `${base} (ref: ${normalizedError.requestId})` : base;
}

const API_ERROR_MESSAGES: Record<string, string> = {
  invalid_credentials: 'Les informations saisies sont incorrectes.',
  unauthorized: 'Votre session a expiré. Reconnectez-vous.',
  forbidden: 'Vous ne pouvez pas effectuer cette action.',
  validation_failed: 'Vérifiez les informations saisies.',
  not_found: 'La ressource demandée est introuvable.',
  conflict: 'Cette action entre en conflit avec une modification récente.',
  database_error: 'Une erreur est survenue. Veuillez réessayer.',
  internal_error: 'Une erreur est survenue. Veuillez réessayer.',
  email_error: "L'envoi de l'email a échoué. Veuillez réessayer.",
  email_not_configured: "L'envoi d'emails n'est pas encore configuré.",
  step_up_required: 'Confirmez votre identité pour continuer.',
  step_up_expired: 'La vérification a expiré. Recommencez.',
  invalid_totp_code: "Le code d'authentification est incorrect.",
  invalid_recovery_code: 'Le code de récupération est incorrect.',
  webauthn_auth_failed:
    "Votre clé d'accès n'a pas pu être vérifiée. Veuillez réessayer ou choisir une autre méthode de connexion.",
  invalid_email_mfa_code: 'Le code reçu par email est incorrect ou a expiré.',
  email_mfa_not_configured: "La vérification par email n'est pas disponible pour ce compte.",
  email_mfa_requires_verified_email: 'Ajoutez un email vérifié avant d’utiliser cette méthode.',
  email_mfa_cannot_be_removed: 'La vérification par email est requise et ne peut pas être retirée.',
  mfa_factor_not_found: 'Cette méthode de vérification est introuvable.',
  challenge_locked: 'Trop de tentatives. Réessayez dans quelques minutes.',
  password_exposed: 'Ce mot de passe est apparu dans une fuite de données. Choisissez-en un autre.',
  password_compromised:
    'Ce mot de passe est apparu dans une fuite de données. Réinitialisez-le avant de vous connecter.',
  risk_policy_blocked: 'Connexion temporairement bloquée pour protéger votre compte.',
  email_already_exists: 'Cet email est déjà utilisé.',
  primary_email_cannot_be_deleted: "L'email principal ne peut pas être supprimé.",
  email_not_verified: 'Vérifiez cet email avant de continuer.',
  email_not_mfa_eligible: "Cet email n'est pas encore disponible pour la vérification par email.",
};

function clientRuntimeErrorMessage(error: ClientRuntimeError, fallback: string): string {
  const code = readApiErrorCode(error.body);
  if (code && API_ERROR_MESSAGES[code]) {
    return API_ERROR_MESSAGES[code];
  }

  if (error.kind === 'api') {
    return httpStatusMessage(error.status, fallback);
  }

  return error.message || fallback;
}

function readApiErrorCode(body: unknown): string | null {
  if (!body || typeof body !== 'object' || !('error' in body)) {
    return null;
  }

  const envelope = body as { error?: unknown };
  if (!envelope.error || typeof envelope.error !== 'object' || !('code' in envelope.error)) {
    return null;
  }

  const error = envelope.error as { code?: unknown };
  return typeof error.code === 'string' ? error.code : null;
}

function httpStatusMessage(status: number | undefined, fallback: string): string {
  if (status === 400) {
    return 'Vérifiez les informations saisies.';
  }
  if (status === 401) {
    return 'Votre session a expiré. Reconnectez-vous.';
  }
  if (status === 403) {
    return 'Vous ne pouvez pas effectuer cette action.';
  }
  if (status === 404) {
    return 'La ressource demandée est introuvable.';
  }
  if (status === 409) {
    return 'Cette action entre en conflit avec une modification récente.';
  }
  if (status && status >= 500) {
    return 'Une erreur est survenue. Veuillez réessayer.';
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
