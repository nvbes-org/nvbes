import {
  ACCEPT_ALL_CONSENT,
  CATEGORY_POSTHOG_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  POSTHOG_PURPOSE_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
  cloneConsent,
  getPostHogConsent,
  getTrackingConsent,
  hasAnyOptionalConsent,
  isCategoryAccepted,
  isPostHogPurposeAccepted,
  isVendorAccepted,
  persistTrackingConsent,
  readTrackingConsentStoredValue,
  type CookieConsentState,
  type TrackingConsentStoredValue,
} from './tracking-consent.storage';
import { HttpError } from '@nvbes/http-client';

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

const BACKEND_SYNC_SOURCE = 'identity-web:sync:backend';

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
  const analyticsGranted =
    hasActiveConsent(activeVersions, 'cookie_consent_analytics') ||
    hasActiveConsent(activeVersions, 'cookie_consent_vendor_posthog') ||
    Object.values(POSTHOG_PURPOSE_CONSENT_TYPES).some((consentType) =>
      hasActiveConsent(activeVersions, consentType),
    );
  const performanceGranted =
    hasActiveConsent(activeVersions, 'cookie_consent_performance') ||
    hasActiveConsent(activeVersions, 'cookie_consent_vendor_sentry') ||
    hasActiveConsent(activeVersions, POSTHOG_PURPOSE_CONSENT_TYPES.errorTracking);
  const posthog = {
    productAnalytics: hasActiveConsent(
      activeVersions,
      POSTHOG_PURPOSE_CONSENT_TYPES.productAnalytics,
    ),
    autocaptureHeatmaps: hasActiveConsent(
      activeVersions,
      POSTHOG_PURPOSE_CONSENT_TYPES.autocaptureHeatmaps,
    ),
    sessionReplay: hasActiveConsent(activeVersions, POSTHOG_PURPOSE_CONSENT_TYPES.sessionReplay),
    surveysFeedback: hasActiveConsent(
      activeVersions,
      POSTHOG_PURPOSE_CONSENT_TYPES.surveysFeedback,
    ),
    errorTracking: hasActiveConsent(activeVersions, POSTHOG_PURPOSE_CONSENT_TYPES.errorTracking),
    featureFlags: hasActiveConsent(activeVersions, POSTHOG_PURPOSE_CONSENT_TYPES.featureFlags),
  };

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
        hasActiveConsent(activeVersions, 'cookie_consent_vendor_stripe') ||
        DEFAULT_CONSENT.vendors.stripe,
      identity:
        hasActiveConsent(activeVersions, 'cookie_consent_vendor_identity') ||
        DEFAULT_CONSENT.vendors.identity,
      posthog:
        hasActiveConsent(activeVersions, 'cookie_consent_vendor_posthog') || analyticsGranted,
      sentry:
        hasActiveConsent(activeVersions, 'cookie_consent_vendor_sentry') || performanceGranted,
    },
    posthog,
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

  if (granted && !versions.has('v3')) {
    promises.push(client.grantConsent(consentType, 'v3'));
  }

  for (const version of versions) {
    if (!granted || version !== 'v3') {
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
    essentials: 'cookie_consent_essentials',
    analytics: 'cookie_consent_analytics',
    performance: 'cookie_consent_performance',
  } as const satisfies Record<keyof CookieConsentState['categories'], string>;

  const vendorMapping = {
    stripe: 'cookie_consent_vendor_stripe',
    identity: 'cookie_consent_vendor_identity',
    posthog: 'cookie_consent_vendor_posthog',
    sentry: 'cookie_consent_vendor_sentry',
  } as const satisfies Record<keyof CookieConsentState['vendors'], string>;

  for (const [category, consentType] of Object.entries(categoryMapping)) {
    queueConsentSync(
      promises,
      activeVersions,
      consentType,
      consent.categories[category as keyof CookieConsentState['categories']],
      client,
    );
  }

  for (const [vendor, consentType] of Object.entries(vendorMapping)) {
    const granted =
      vendor === 'posthog'
        ? consent.vendors.posthog
        : consent.vendors[vendor as keyof CookieConsentState['vendors']];
    queueConsentSync(promises, activeVersions, consentType, granted, client);
  }

  for (const [purpose, consentType] of Object.entries(POSTHOG_PURPOSE_CONSENT_TYPES)) {
    queueConsentSync(
      promises,
      activeVersions,
      consentType,
      consent.posthog[purpose as keyof CookieConsentState['posthog']],
      client,
    );
  }

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

async function reconcileTrackingConsent(client: TrackingConsentClient): Promise<void> {
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

  const backendChangedAt = latestBackendConsentChangeAt(backendConsents);
  const backendConsent = backendConsentState(backendConsents);
  const storedValue = readTrackingConsentStoredValue();
  const localConsent = getTrackingConsent();
  const localChangedAt = storedValue ? Date.parse(storedValue.savedAt) : Number.NaN;

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
  CATEGORY_POSTHOG_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  POSTHOG_PURPOSE_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
  cloneConsent,
  getPostHogConsent,
  getTrackingConsent,
  isCategoryAccepted,
  isPostHogPurposeAccepted,
  isVendorAccepted,
  type CookieConsentState,
  type TrackingConsentStoredValue,
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

  const syncTrackingConsent = async () => {
    await reconcileTrackingConsent(identityClient);
  };

  return {
    getTrackingConsent,
    getPostHogConsent,
    isPostHogPurposeAccepted,
    isVendorAccepted,
    isCategoryAccepted,
    setTrackingConsent,
    syncTrackingConsent,
  };
}
