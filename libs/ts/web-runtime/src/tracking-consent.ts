import { HttpError } from '@nvbes/http-client';
import {
  ACCEPT_ALL_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  type CookieConsentState,
  cloneConsent,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  getAnalyticsConsent,
  getTrackingConsent,
  hasAnyOptionalConsent,
  isAnalyticsPurposeAccepted,
  isCategoryAccepted,
  isVendorAccepted,
  LEGACY_VENDOR_CONSENT_TYPES,
  persistTrackingConsent,
  readTrackingConsentStoredValue,
  TRACKING_CONSENT_CHANGED_EVENT,
  type TrackingConsentStoredValue,
  VENDOR_CONSENT_TYPES,
} from './tracking-consent.storage';

export type BackendConsent = {
  consent_type: string;
  document_version: string;
  granted_at: string;
  revoked_at?: string | null;
};

export type TrackingConsentClient = {
  listConsents: (options?: { signal?: AbortSignal }) => Promise<BackendConsent[]>;
  grantConsent: (consentType: string, documentVersion: string) => Promise<unknown>;
  revokeConsent: (consentType: string, documentVersion: string) => Promise<unknown>;
  isAuthenticated?: () => boolean | Promise<boolean>;
};

export type TrackingConsentSyncOptions = {
  sourceOfTruth?: 'newest' | 'backend';
};

const BACKEND_SYNC_SOURCE = 'tracking-consent:sync:backend';
export const TRACKING_CONSENT_DOCUMENT_VERSION = 'cookie-notice-2026-07-20';

function activeVersionsByConsentType(consents: BackendConsent[]): Map<string, Set<string>> {
  const active = new Map<string, Set<string>>();
  for (const consent of consents) {
    if (consent.revoked_at) continue;

    const versions = active.get(consent.consent_type) ?? new Set<string>();
    versions.add(consent.document_version);
    active.set(consent.consent_type, versions);
  }
  return active;
}

function consentChangeTimestamp(consent: BackendConsent): number {
  const grantedAt = Date.parse(consent.granted_at);
  const revokedAt = consent.revoked_at ? Date.parse(consent.revoked_at) : Number.NaN;

  if (Number.isFinite(revokedAt)) {
    return Math.max(grantedAt, revokedAt);
  }

  return grantedAt;
}

function latestBackendConsentChangeAt(consents: BackendConsent[]): number | null {
  let latest = Number.NEGATIVE_INFINITY;
  for (const consent of consents) {
    const timestamp = consentChangeTimestamp(consent);
    if (Number.isFinite(timestamp) && timestamp > latest) {
      latest = timestamp;
    }
  }

  return Number.isFinite(latest) ? latest : null;
}

function hasActiveConsent(activeVersions: Map<string, Set<string>>, consentType: string): boolean {
  return (activeVersions.get(consentType)?.size ?? 0) > 0;
}

function backendConsentState(consents: BackendConsent[]): CookieConsentState {
  const activeVersions = activeVersionsByConsentType(consents);
  const hasPurposeRecords = consents.some((consent) =>
    Object.values(ANALYTICS_PURPOSE_CONSENT_TYPES).includes(
      consent.consent_type as (typeof ANALYTICS_PURPOSE_CONSENT_TYPES)[keyof typeof ANALYTICS_PURPOSE_CONSENT_TYPES],
    ),
  );
  const legacyAnalyticsGranted =
    hasActiveConsent(activeVersions, 'cookie_consent_analytics') ||
    hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.posthog) ||
    hasActiveConsent(activeVersions, LEGACY_VENDOR_CONSENT_TYPES.analytics);
  const legacyPerformanceGranted =
    hasActiveConsent(activeVersions, 'cookie_consent_performance') ||
    hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.sentry) ||
    hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.grafana) ||
    hasActiveConsent(activeVersions, LEGACY_VENDOR_CONSENT_TYPES.errorReporting);
  const analytics = {
    productAnalytics: hasPurposeRecords
      ? hasActiveConsent(activeVersions, ANALYTICS_PURPOSE_CONSENT_TYPES.productAnalytics)
      : legacyAnalyticsGranted,
    autocaptureHeatmaps:
      hasPurposeRecords &&
      hasActiveConsent(activeVersions, ANALYTICS_PURPOSE_CONSENT_TYPES.autocaptureHeatmaps),
    sessionReplay:
      hasPurposeRecords &&
      hasActiveConsent(activeVersions, ANALYTICS_PURPOSE_CONSENT_TYPES.sessionReplay),
    surveysFeedback:
      hasPurposeRecords &&
      hasActiveConsent(activeVersions, ANALYTICS_PURPOSE_CONSENT_TYPES.surveysFeedback),
    errorTracking: hasPurposeRecords
      ? hasActiveConsent(activeVersions, ANALYTICS_PURPOSE_CONSENT_TYPES.errorTracking)
      : legacyPerformanceGranted,
    featureFlags:
      hasPurposeRecords &&
      hasActiveConsent(activeVersions, ANALYTICS_PURPOSE_CONSENT_TYPES.featureFlags),
  };
  const analyticsGranted = Object.entries(analytics).some(
    ([purpose, granted]) => purpose !== 'errorTracking' && granted,
  );
  const performanceGranted = analytics.errorTracking;

  return {
    categories: {
      essentials:
        hasActiveConsent(activeVersions, 'cookie_consent_essentials') ||
        DEFAULT_CONSENT.categories.essentials,
      analytics: analyticsGranted,
      performance: performanceGranted,
    },
    vendors: {
      stripe:
        hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.stripe) ||
        DEFAULT_CONSENT.vendors.stripe,
      identity:
        hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.identity) ||
        DEFAULT_CONSENT.vendors.identity,
      cloudflare:
        hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.cloudflare) ||
        DEFAULT_CONSENT.vendors.cloudflare,
      posthog: hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.posthog) || analyticsGranted,
      sentry: hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.sentry) || performanceGranted,
      grafana: hasActiveConsent(activeVersions, VENDOR_CONSENT_TYPES.grafana) || performanceGranted,
    },
    analytics,
  };
}

function queueConsentSync(
  promises: Promise<unknown>[],
  activeVersions: Map<string, Set<string>>,
  consentType: string,
  granted: boolean,
  client: TrackingConsentClient,
): void {
  const versions = activeVersions.get(consentType) ?? new Set<string>();

  if (granted && !versions.has(TRACKING_CONSENT_DOCUMENT_VERSION)) {
    promises.push(client.grantConsent(consentType, TRACKING_CONSENT_DOCUMENT_VERSION));
  }

  for (const version of versions) {
    if (!granted || version !== TRACKING_CONSENT_DOCUMENT_VERSION) {
      promises.push(client.revokeConsent(consentType, version));
    }
  }
}

async function syncLocalConsentToBackend(
  consent: CookieConsentState,
  backendConsents: BackendConsent[],
  client: TrackingConsentClient,
): Promise<void> {
  const activeVersions = activeVersionsByConsentType(backendConsents);
  const promises: Promise<unknown>[] = [];

  const categoryMapping = {
    analytics: 'cookie_consent_analytics',
    performance: 'cookie_consent_performance',
  } as const;

  for (const [category, consentType] of Object.entries(categoryMapping)) {
    queueConsentSync(
      promises,
      activeVersions,
      consentType,
      consent.categories[category as keyof typeof categoryMapping],
      client,
    );
  }

  for (const [vendor, consentType] of Object.entries(VENDOR_CONSENT_TYPES)) {
    if (vendor === 'stripe' || vendor === 'identity' || vendor === 'cloudflare') {
      queueConsentSync(promises, activeVersions, consentType, false, client);
      continue;
    }
    const granted = consent.vendors[vendor as keyof CookieConsentState['vendors']];
    queueConsentSync(promises, activeVersions, consentType, granted, client);
  }

  for (const [purpose, consentType] of Object.entries(ANALYTICS_PURPOSE_CONSENT_TYPES)) {
    queueConsentSync(
      promises,
      activeVersions,
      consentType,
      consent.analytics[purpose as keyof CookieConsentState['analytics']],
      client,
    );
  }

  for (const consentType of Object.values(LEGACY_VENDOR_CONSENT_TYPES)) {
    queueConsentSync(promises, activeVersions, consentType, false, client);
  }
  queueConsentSync(promises, activeVersions, 'cookie_consent_essentials', false, client);

  queueConsentSync(
    promises,
    activeVersions,
    'cookie_consent',
    hasAnyOptionalConsent(consent),
    client,
  );

  if (promises.length > 0) {
    await Promise.all(promises);
  }
}

function dispatchTrackingConsentChange(consent: CookieConsentState, source: string): void {
  window.dispatchEvent(
    new CustomEvent(TRACKING_CONSENT_CHANGED_EVENT, {
      detail: { consent, source },
    }),
  );
}

function isSkippableConsentSyncError(error: unknown): boolean {
  return error instanceof HttpError && (error.status === 401 || error.status === 403);
}

async function reconcileTrackingConsent(
  client: TrackingConsentClient,
  options: TrackingConsentSyncOptions = {},
): Promise<void> {
  if (typeof client.isAuthenticated === 'function' && !(await client.isAuthenticated())) {
    return;
  }

  let backendConsents: BackendConsent[];
  try {
    backendConsents = await client.listConsents();
  } catch (error) {
    if (isSkippableConsentSyncError(error)) {
      return;
    }
    throw error;
  }

  const currentBackendConsents = backendConsents.filter(
    (consent) => consent.document_version === TRACKING_CONSENT_DOCUMENT_VERSION,
  );
  const backendChangedAt = latestBackendConsentChangeAt(currentBackendConsents);
  const backendConsent = backendConsentState(currentBackendConsents);
  const storedValue = readTrackingConsentStoredValue();
  const localConsent = getTrackingConsent();
  const localChangedAt = storedValue ? Date.parse(storedValue.savedAt) : Number.NaN;

  if (options.sourceOfTruth === 'backend') {
    const persistedAt = new Date(backendChangedAt ?? Date.now());
    persistTrackingConsent(backendConsent, BACKEND_SYNC_SOURCE, persistedAt);
    dispatchTrackingConsentChange(backendConsent, BACKEND_SYNC_SOURCE);
    return;
  }

  if (
    backendChangedAt !== null &&
    (!Number.isFinite(localChangedAt) || backendChangedAt > localChangedAt)
  ) {
    persistTrackingConsent(backendConsent, BACKEND_SYNC_SOURCE, new Date(backendChangedAt));
    dispatchTrackingConsentChange(backendConsent, BACKEND_SYNC_SOURCE);
    return;
  }

  if (localConsent) {
    await syncLocalConsentToBackend(localConsent, backendConsents, client);
  }
}

export {
  ACCEPT_ALL_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  type CookieConsentState,
  cloneConsent,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  getAnalyticsConsent,
  getTrackingConsent,
  isAnalyticsPurposeAccepted,
  isCategoryAccepted,
  isVendorAccepted,
  LEGACY_VENDOR_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
  type TrackingConsentStoredValue,
  VENDOR_CONSENT_TYPES,
};

export function createTrackingConsentApi({
  identityClient,
  defaultSource,
}: {
  identityClient: TrackingConsentClient;
  defaultSource: string;
}) {
  const setTrackingConsent = (consent: CookieConsentState, source = defaultSource) => {
    const normalized = cloneConsent(consent);
    persistTrackingConsent(normalized, source);
    dispatchTrackingConsentChange(normalized, source);

    void (async () => {
      try {
        await reconcileTrackingConsent(identityClient);
      } catch {
        // Ignore sync errors after local persistence.
      }
    })();
  };

  const prepareTrackingConsentSubjectSwitch = (
    source = `${defaultSource}:subject-switch`,
  ): void => {
    const suspendedConsent = cloneConsent(DECLINE_ALL_CONSENT);
    persistTrackingConsent(suspendedConsent, source);
    dispatchTrackingConsentChange(suspendedConsent, source);
  };

  const syncTrackingConsent = async (options?: TrackingConsentSyncOptions) => {
    await reconcileTrackingConsent(identityClient, options);
  };

  return {
    getTrackingConsent,
    getAnalyticsConsent,
    isAnalyticsPurposeAccepted,
    isVendorAccepted,
    isCategoryAccepted,
    setTrackingConsent,
    prepareTrackingConsentSubjectSwitch,
    syncTrackingConsent,
  };
}
