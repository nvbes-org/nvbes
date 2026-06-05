import { identityClient, type UserConsent } from '@nvbes/identity-client';
import {
  ALL_POSTHOG_CONSENT,
  EMPTY_POSTHOG_CONSENT,
  hasAnyPostHogConsent,
  type PostHogPurposeConsent,
} from '@nvbes/web-runtime/posthog';

export interface CookieConsentState {
  categories: {
    essentials: boolean;
    analytics: boolean;
    performance: boolean;
  };
  vendors: {
    stripe: boolean;
    identity: boolean;
    posthog: boolean;
    sentry: boolean;
  };
  posthog: PostHogPurposeConsent;
}

export interface TrackingConsentStoredValue {
  version: 3;
  savedAt: string;
  expiresAt: string;
  source: string;
  consent: CookieConsentState;
}

type ConsentCategory = keyof CookieConsentState['categories'];
type ConsentVendor = keyof CookieConsentState['vendors'];
type PostHogPurpose = keyof PostHogPurposeConsent;

type LegacyConsentCandidate = {
  categories?: Partial<Record<ConsentCategory | 'marketing', boolean>>;
  vendors?: Partial<Record<ConsentVendor | 'marketingVendor', boolean>>;
  posthog?: Partial<Record<PostHogPurpose, boolean>>;
};

type StoredConsentCandidate = {
  version?: number;
  savedAt?: string;
  expiresAt?: string;
  source?: string;
  consent?: unknown;
};

type ActiveConsent = Pick<UserConsent, 'consent_type' | 'document_version' | 'revoked_at'>;

export const TRACKING_CONSENT_CHANGED_EVENT = 'nvbes:tracking-consent-changed';
const CONSENT_TTL_DAYS = 183;
const DOCUMENT_VERSION = 'v3';

export const CATEGORY_VENDORS_MAP = {
  essentials: ['stripe', 'identity'],
  analytics: ['posthog'],
  performance: ['sentry'],
} as const;

export const CATEGORY_POSTHOG_PURPOSES_MAP = {
  essentials: [],
  analytics: [
    'productAnalytics',
    'autocaptureHeatmaps',
    'sessionReplay',
    'surveysFeedback',
    'featureFlags',
  ],
  performance: ['errorTracking'],
} as const satisfies Record<ConsentCategory, readonly PostHogPurpose[]>;

export const POSTHOG_PURPOSE_CONSENT_TYPES = {
  productAnalytics: 'posthog_product_analytics',
  autocaptureHeatmaps: 'posthog_autocapture_heatmaps',
  sessionReplay: 'posthog_session_replay',
  surveysFeedback: 'posthog_surveys_feedback',
  errorTracking: 'posthog_error_tracking',
  featureFlags: 'posthog_feature_flags',
} as const satisfies Record<PostHogPurpose, string>;

export const DEFAULT_CONSENT: CookieConsentState = {
  categories: {
    essentials: true,
    analytics: false,
    performance: false,
  },
  vendors: {
    stripe: true,
    identity: true,
    posthog: false,
    sentry: false,
  },
  posthog: EMPTY_POSTHOG_CONSENT,
};

export const ACCEPT_ALL_CONSENT: CookieConsentState = {
  categories: {
    essentials: true,
    analytics: true,
    performance: true,
  },
  vendors: {
    stripe: true,
    identity: true,
    posthog: true,
    sentry: true,
  },
  posthog: ALL_POSTHOG_CONSENT,
};

export const DECLINE_ALL_CONSENT: CookieConsentState = {
  categories: {
    essentials: true,
    analytics: false,
    performance: false,
  },
  vendors: {
    stripe: true,
    identity: true,
    posthog: false,
    sentry: false,
  },
  posthog: EMPTY_POSTHOG_CONSENT,
};

const STORAGE_KEY_V3 = 'nvbes.tracking-consent.v3';
const STORAGE_KEY_V2 = 'nvbes.tracking-consent.v2';
const STORAGE_KEY_V1 = 'nvbes.tracking-consent.v1';

function consentExpiry(savedAt: Date): string {
  const expiresAt = new Date(savedAt);
  expiresAt.setDate(expiresAt.getDate() + CONSENT_TTL_DAYS);
  return expiresAt.toISOString();
}

function clonePostHogConsent(consent: PostHogPurposeConsent): PostHogPurposeConsent {
  return { ...consent };
}

function cloneConsent(consent: CookieConsentState): CookieConsentState {
  return {
    categories: { ...consent.categories },
    vendors: { ...consent.vendors },
    posthog: clonePostHogConsent(consent.posthog),
  };
}

function createStoredConsent(
  consent: CookieConsentState,
  source: string,
  savedAt = new Date(),
): TrackingConsentStoredValue {
  return {
    version: 3,
    savedAt: savedAt.toISOString(),
    expiresAt: consentExpiry(savedAt),
    source,
    consent: cloneConsent(consent),
  };
}

function booleanValue(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function normalizePostHogConsent(
  candidate: LegacyConsentCandidate,
  legacyProductOnly: boolean,
): PostHogPurposeConsent {
  const legacyPostHogVendor = booleanValue(candidate.vendors?.posthog, false);
  if (legacyProductOnly) {
    return {
      ...EMPTY_POSTHOG_CONSENT,
      productAnalytics: legacyPostHogVendor,
    };
  }

  return {
    productAnalytics: booleanValue(candidate.posthog?.productAnalytics, legacyPostHogVendor),
    autocaptureHeatmaps: booleanValue(candidate.posthog?.autocaptureHeatmaps, false),
    sessionReplay: booleanValue(candidate.posthog?.sessionReplay, false),
    surveysFeedback: booleanValue(candidate.posthog?.surveysFeedback, false),
    errorTracking: booleanValue(candidate.posthog?.errorTracking, false),
    featureFlags: booleanValue(candidate.posthog?.featureFlags, false),
  };
}

function normalizeConsent(value: unknown, legacyProductOnly: boolean): CookieConsentState | null {
  if (!isObject(value)) {
    return null;
  }

  const candidate = value as LegacyConsentCandidate;
  const posthog = normalizePostHogConsent(candidate, legacyProductOnly);
  const hasPostHog = hasAnyPostHogConsent(posthog);

  const analyticsGranted =
    booleanValue(candidate.categories?.analytics, false) ||
    posthog.productAnalytics ||
    posthog.autocaptureHeatmaps ||
    posthog.sessionReplay ||
    posthog.surveysFeedback ||
    posthog.featureFlags;
  const performanceGranted =
    booleanValue(candidate.categories?.performance, false) ||
    booleanValue(candidate.vendors?.sentry, false) ||
    posthog.errorTracking;

  return {
    categories: {
      essentials: true,
      analytics: analyticsGranted,
      performance: performanceGranted,
    },
    vendors: {
      stripe: booleanValue(candidate.vendors?.stripe, true),
      identity: booleanValue(candidate.vendors?.identity, true),
      posthog: hasPostHog,
      sentry: booleanValue(candidate.vendors?.sentry, performanceGranted),
    },
    posthog,
  };
}

function parseStoredConsent(value: string, legacyProductOnly: boolean): CookieConsentState | null {
  const parsed = JSON.parse(value) as StoredConsentCandidate | CookieConsentState;
  const expiresAt = isObject(parsed) && 'expiresAt' in parsed ? Date.parse(String(parsed.expiresAt)) : Number.NaN;

  if (Number.isFinite(expiresAt) && expiresAt <= Date.now()) {
    window.localStorage.removeItem(STORAGE_KEY_V3);
    window.localStorage.removeItem(STORAGE_KEY_V2);
    window.localStorage.removeItem(STORAGE_KEY_V1);
    return null;
  }

  if (isObject(parsed) && 'consent' in parsed) {
    return normalizeConsent(parsed.consent, legacyProductOnly);
  }

  return normalizeConsent(parsed, legacyProductOnly);
}

function legacyV1Consent(accepted: boolean): CookieConsentState {
  if (!accepted) {
    return cloneConsent(DECLINE_ALL_CONSENT);
  }

  return {
    categories: {
      essentials: true,
      analytics: true,
      performance: true,
    },
    vendors: {
      stripe: true,
      identity: true,
      posthog: true,
      sentry: true,
    },
    posthog: {
      ...EMPTY_POSTHOG_CONSENT,
      productAnalytics: true,
    },
  };
}

function persistMigratedConsent(consent: CookieConsentState, source: string): void {
  window.localStorage.setItem(STORAGE_KEY_V3, JSON.stringify(createStoredConsent(consent, source)));
}

export function getTrackingConsent(): CookieConsentState | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const valueV3 = window.localStorage.getItem(STORAGE_KEY_V3);
  if (valueV3) {
    try {
      return parseStoredConsent(valueV3, false);
    } catch {
      window.localStorage.removeItem(STORAGE_KEY_V3);
    }
  }

  const valueV2 = window.localStorage.getItem(STORAGE_KEY_V2);
  if (valueV2) {
    try {
      const migrated = parseStoredConsent(valueV2, true);
      if (migrated) {
        persistMigratedConsent(migrated, 'legacy-v2-migration');
        return migrated;
      }
    } catch {
      window.localStorage.removeItem(STORAGE_KEY_V2);
    }
  }

  const valueV1 = window.localStorage.getItem(STORAGE_KEY_V1);
  if (valueV1 === 'accepted' || valueV1 === 'declined') {
    const migrated = legacyV1Consent(valueV1 === 'accepted');
    persistMigratedConsent(migrated, 'legacy-v1-migration');
    return migrated;
  }

  return null;
}

export function getPostHogConsent(): PostHogPurposeConsent {
  const consent = getTrackingConsent();
  return clonePostHogConsent(consent?.posthog ?? EMPTY_POSTHOG_CONSENT);
}

export function isPostHogPurposeAccepted(purpose: PostHogPurpose): boolean {
  return getPostHogConsent()[purpose] === true;
}

export function isVendorAccepted(vendor: ConsentVendor): boolean {
  const consent = getTrackingConsent();
  if (!consent) return false;
  if (vendor === 'posthog') return hasAnyPostHogConsent(consent.posthog);
  return consent.vendors[vendor] || false;
}

export function isCategoryAccepted(category: ConsentCategory): boolean {
  const consent = getTrackingConsent();
  if (!consent) return false;
  return consent.categories[category] || false;
}

function hasAnyOptionalConsent(consent: CookieConsentState): boolean {
  return (
    consent.categories.analytics ||
    consent.categories.performance ||
    hasAnyPostHogConsent(consent.posthog) ||
    consent.vendors.sentry
  );
}

export function setTrackingConsent(consent: CookieConsentState, source = 'identity-web') {
  const normalized = normalizeConsent(consent, false) ?? cloneConsent(DEFAULT_CONSENT);
  window.localStorage.setItem(STORAGE_KEY_V3, JSON.stringify(createStoredConsent(normalized, source)));

  const hasAnyOptional = hasAnyOptionalConsent(normalized);
  window.localStorage.setItem(STORAGE_KEY_V1, hasAnyOptional ? 'accepted' : 'declined');

  window.dispatchEvent(
    new CustomEvent(TRACKING_CONSENT_CHANGED_EVENT, {
      detail: { consent: normalized, source },
    }),
  );

  void (async () => {
    try {
      await syncWithBackend(normalized);
    } catch {
      // Ignore if not logged in or endpoint fails.
    }
  })();
}

function activeVersionsByConsentType(consents: ActiveConsent[]): Map<string, Set<string>> {
  const active = new Map<string, Set<string>>();
  for (const consent of consents) {
    if (consent.revoked_at) continue;

    const versions = active.get(consent.consent_type) ?? new Set<string>();
    versions.add(consent.document_version);
    active.set(consent.consent_type, versions);
  }
  return active;
}

function queueConsentSync(
  promises: Promise<unknown>[],
  activeVersions: Map<string, Set<string>>,
  consentType: string,
  granted: boolean,
): void {
  const versions = activeVersions.get(consentType) ?? new Set<string>();

  if (granted && !versions.has(DOCUMENT_VERSION)) {
    promises.push(identityClient.grantConsent(consentType, DOCUMENT_VERSION));
  }

  for (const version of versions) {
    if (!granted || version !== DOCUMENT_VERSION) {
      promises.push(identityClient.revokeConsent(consentType, version));
    }
  }
}

async function syncWithBackend(consent: CookieConsentState) {
  try {
    const consents = await identityClient.listConsents();
    const activeConsents = activeVersionsByConsentType(consents);

    const categoryMapping = {
      essentials: 'cookie_consent_essentials',
      analytics: 'cookie_consent_analytics',
      performance: 'cookie_consent_performance',
    } as const satisfies Record<ConsentCategory, string>;

    const vendorMapping = {
      stripe: 'cookie_consent_vendor_stripe',
      identity: 'cookie_consent_vendor_identity',
      posthog: 'cookie_consent_vendor_posthog',
      sentry: 'cookie_consent_vendor_sentry',
    } as const satisfies Record<ConsentVendor, string>;

    const promises: Promise<unknown>[] = [];

    for (const [category, consentType] of Object.entries(categoryMapping)) {
      queueConsentSync(promises, activeConsents, consentType, consent.categories[category as ConsentCategory]);
    }

    for (const [vendor, consentType] of Object.entries(vendorMapping)) {
      const granted =
        vendor === 'posthog'
          ? hasAnyPostHogConsent(consent.posthog)
          : consent.vendors[vendor as ConsentVendor];
      queueConsentSync(promises, activeConsents, consentType, granted);
    }

    for (const [purpose, consentType] of Object.entries(POSTHOG_PURPOSE_CONSENT_TYPES)) {
      queueConsentSync(
        promises,
        activeConsents,
        consentType,
        consent.posthog[purpose as PostHogPurpose],
      );
    }

    queueConsentSync(
      promises,
      activeConsents,
      'cookie_consent',
      hasAnyOptionalConsent(consent),
    );

    if (promises.length > 0) {
      await Promise.all(promises);
    }
  } catch {
    // Suppress errors during client updates (e.g. offline, unauthenticated).
  }
}

export async function syncTrackingConsent() {
  const current = getTrackingConsent();
  if (current) {
    await syncWithBackend(current);
  }
}
