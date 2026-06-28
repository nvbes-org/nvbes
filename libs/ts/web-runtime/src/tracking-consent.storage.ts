import {
  ALL_ANALYTICS_CONSENT,
  EMPTY_ANALYTICS_CONSENT,
  type AnalyticsPurposeConsent,
} from './analytics';
import { getSafeLocalStorage } from './safe-storage';

export interface CookieConsentState {
  categories: {
    essentials: boolean;
    analytics: boolean;
    performance: boolean;
  };
  vendors: {
    stripe: boolean;
    identity: boolean;
    cloudflare: boolean;
    posthog: boolean;
    sentry: boolean;
    grafana: boolean;
  };
  analytics: AnalyticsPurposeConsent;
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
type AnalyticsPurpose = keyof AnalyticsPurposeConsent;

type LegacyConsentCandidate = {
  categories?: Partial<Record<ConsentCategory | 'marketing', boolean>>;
  vendors?: Partial<
    Record<ConsentVendor | 'analytics' | 'errorReporting' | 'marketingVendor', boolean>
  >;
  analytics?: Partial<Record<AnalyticsPurpose, boolean>>;
};

type StoredConsentCandidate = {
  version?: number;
  savedAt?: string;
  expiresAt?: string;
  source?: string;
  consent?: unknown;
};

export const TRACKING_CONSENT_CHANGED_EVENT = 'nvbes:tracking-consent-changed';
const CONSENT_TTL_DAYS = 183;
const STORAGE_KEY_V3 = 'nvbes.tracking-consent.v3';
const STORAGE_KEY_V2 = 'nvbes.tracking-consent.v2';
const STORAGE_KEY_V1 = 'nvbes.tracking-consent.v1';

export const CATEGORY_VENDORS_MAP = {
  essentials: ['stripe', 'identity', 'cloudflare'],
  analytics: ['posthog'],
  performance: ['sentry', 'grafana'],
} as const;

export const CATEGORY_ANALYTICS_PURPOSES_MAP = {
  essentials: [],
  analytics: [
    'productAnalytics',
    'autocaptureHeatmaps',
    'sessionReplay',
    'surveysFeedback',
    'featureFlags',
  ],
  performance: ['errorTracking'],
} as const satisfies Record<ConsentCategory, readonly AnalyticsPurpose[]>;

export const ANALYTICS_PURPOSE_CONSENT_TYPES = {
  productAnalytics: 'analytics_product_analytics',
  autocaptureHeatmaps: 'analytics_autocapture_heatmaps',
  sessionReplay: 'analytics_session_replay',
  surveysFeedback: 'analytics_surveys_feedback',
  errorTracking: 'analytics_error_tracking',
  featureFlags: 'analytics_feature_flags',
} as const satisfies Record<AnalyticsPurpose, string>;

export const VENDOR_CONSENT_TYPES = {
  stripe: 'cookie_consent_vendor_stripe',
  identity: 'cookie_consent_vendor_identity',
  cloudflare: 'cookie_consent_vendor_cloudflare',
  posthog: 'cookie_consent_vendor_posthog',
  sentry: 'cookie_consent_vendor_sentry',
  grafana: 'cookie_consent_vendor_grafana',
} as const satisfies Record<ConsentVendor, string>;

export const LEGACY_VENDOR_CONSENT_TYPES = {
  analytics: 'cookie_consent_vendor_analytics',
  errorReporting: 'cookie_consent_vendor_error_reporting',
} as const;

function analyticsConsentForState(
  categories: CookieConsentState['categories'],
  vendors: CookieConsentState['vendors'],
): AnalyticsPurposeConsent {
  const analyticsGranted = categories.analytics || vendors.posthog;
  const performanceGranted = categories.performance || vendors.sentry || vendors.grafana;

  return {
    productAnalytics: analyticsGranted,
    autocaptureHeatmaps: analyticsGranted,
    sessionReplay: analyticsGranted,
    surveysFeedback: analyticsGranted,
    featureFlags: analyticsGranted,
    errorTracking: performanceGranted,
  };
}

export const DEFAULT_CONSENT: CookieConsentState = {
  categories: {
    essentials: true,
    analytics: false,
    performance: false,
  },
  vendors: {
    stripe: true,
    identity: true,
    cloudflare: true,
    posthog: false,
    sentry: false,
    grafana: false,
  },
  analytics: EMPTY_ANALYTICS_CONSENT,
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
    cloudflare: true,
    posthog: true,
    sentry: true,
    grafana: true,
  },
  analytics: ALL_ANALYTICS_CONSENT,
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
    cloudflare: true,
    posthog: false,
    sentry: false,
    grafana: false,
  },
  analytics: EMPTY_ANALYTICS_CONSENT,
};

function consentExpiry(savedAt: Date): string {
  const expiresAt = new Date(savedAt);
  expiresAt.setDate(expiresAt.getDate() + CONSENT_TTL_DAYS);
  return expiresAt.toISOString();
}

export function cloneConsent(consent: CookieConsentState): CookieConsentState {
  return {
    categories: { ...consent.categories },
    vendors: { ...consent.vendors },
    analytics: { ...consent.analytics },
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

function normalizeAnalyticsConsent(
  candidate: LegacyConsentCandidate,
  legacyProductOnly: boolean,
): AnalyticsPurposeConsent {
  const legacyAnalyticsVendor = booleanValue(candidate.vendors?.analytics, false);
  if (legacyProductOnly) {
    return {
      ...EMPTY_ANALYTICS_CONSENT,
      productAnalytics: legacyAnalyticsVendor,
    };
  }

  return {
    productAnalytics: booleanValue(candidate.analytics?.productAnalytics, legacyAnalyticsVendor),
    autocaptureHeatmaps: booleanValue(candidate.analytics?.autocaptureHeatmaps, false),
    sessionReplay: booleanValue(candidate.analytics?.sessionReplay, false),
    surveysFeedback: booleanValue(candidate.analytics?.surveysFeedback, false),
    errorTracking: booleanValue(candidate.analytics?.errorTracking, false),
    featureFlags: booleanValue(candidate.analytics?.featureFlags, false),
  };
}

function normalizeConsent(value: unknown, legacyProductOnly: boolean): CookieConsentState | null {
  if (!isObject(value)) {
    return null;
  }

  const candidate = value as LegacyConsentCandidate;
  const legacyAnalytics = normalizeAnalyticsConsent(candidate, legacyProductOnly);

  const analyticsGranted =
    booleanValue(candidate.categories?.analytics, false) ||
    booleanValue(candidate.vendors?.posthog, false) ||
    booleanValue(candidate.vendors?.analytics, false) ||
    legacyAnalytics.productAnalytics ||
    legacyAnalytics.autocaptureHeatmaps ||
    legacyAnalytics.sessionReplay ||
    legacyAnalytics.surveysFeedback ||
    legacyAnalytics.featureFlags;
  const performanceGranted =
    booleanValue(candidate.categories?.performance, false) ||
    booleanValue(candidate.vendors?.sentry, false) ||
    booleanValue(candidate.vendors?.grafana, false) ||
    booleanValue(candidate.vendors?.errorReporting, false) ||
    legacyAnalytics.errorTracking;

  const categories = {
    essentials: true,
    analytics: analyticsGranted,
    performance: performanceGranted,
  };
  const vendors = {
    stripe: booleanValue(candidate.vendors?.stripe, true),
    identity: booleanValue(candidate.vendors?.identity, true),
    cloudflare: booleanValue(candidate.vendors?.cloudflare, true),
    posthog: booleanValue(candidate.vendors?.posthog, analyticsGranted),
    sentry: booleanValue(candidate.vendors?.sentry, performanceGranted),
    grafana: booleanValue(candidate.vendors?.grafana, performanceGranted),
  };

  return {
    categories,
    vendors,
    analytics: analyticsConsentForState(categories, vendors),
  };
}

function parseStoredConsent(value: string, legacyProductOnly: boolean): CookieConsentState | null {
  const parsed = JSON.parse(value) as StoredConsentCandidate | CookieConsentState;
  const expiresAt =
    isObject(parsed) && 'expiresAt' in parsed ? Date.parse(String(parsed.expiresAt)) : Number.NaN;

  if (Number.isFinite(expiresAt) && expiresAt <= Date.now()) {
    clearStoredTrackingConsentVersions();
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
      cloudflare: true,
      posthog: true,
      sentry: true,
      grafana: true,
    },
    analytics: ALL_ANALYTICS_CONSENT,
  };
}

function persistMigratedConsent(consent: CookieConsentState, source: string): void {
  getSafeLocalStorage().setItem(
    STORAGE_KEY_V3,
    JSON.stringify(createStoredConsent(consent, source)),
  );
}

export function readTrackingConsentStoredValue(): TrackingConsentStoredValue | null {
  const storage = getSafeLocalStorage();
  const value = storage.getItem(STORAGE_KEY_V3);
  if (!value) {
    return null;
  }

  try {
    const parsed = JSON.parse(value) as TrackingConsentStoredValue;
    const expiresAt = Date.parse(parsed.expiresAt);
    if (Number.isFinite(expiresAt) && expiresAt <= Date.now()) {
      clearStoredTrackingConsentVersions();
      return null;
    }

    return parsed;
  } catch {
    storage.removeItem(STORAGE_KEY_V3);
    return null;
  }
}

export function getTrackingConsent(): CookieConsentState | null {
  const storage = getSafeLocalStorage();

  const valueV3 = storage.getItem(STORAGE_KEY_V3);
  if (valueV3) {
    try {
      return parseStoredConsent(valueV3, false);
    } catch {
      storage.removeItem(STORAGE_KEY_V3);
    }
  }

  const valueV2 = storage.getItem(STORAGE_KEY_V2);
  if (valueV2) {
    try {
      const migrated = parseStoredConsent(valueV2, true);
      if (migrated) {
        persistMigratedConsent(migrated, 'legacy-v2-migration');
        return migrated;
      }
    } catch {
      storage.removeItem(STORAGE_KEY_V2);
    }
  }

  const valueV1 = storage.getItem(STORAGE_KEY_V1);
  if (valueV1 === 'accepted' || valueV1 === 'declined') {
    const migrated = legacyV1Consent(valueV1 === 'accepted');
    persistMigratedConsent(migrated, 'legacy-v1-migration');
    return migrated;
  }

  return null;
}

export function getAnalyticsConsent(): AnalyticsPurposeConsent {
  const consent = getTrackingConsent();
  return { ...(consent?.analytics ?? EMPTY_ANALYTICS_CONSENT) };
}

export function isAnalyticsPurposeAccepted(purpose: AnalyticsPurpose): boolean {
  return getAnalyticsConsent()[purpose] === true;
}

export function isVendorAccepted(vendor: ConsentVendor): boolean {
  const consent = getTrackingConsent();
  if (!consent) return false;
  return consent.vendors[vendor] || false;
}

export function isCategoryAccepted(category: ConsentCategory): boolean {
  const consent = getTrackingConsent();
  if (!consent) return false;
  return consent.categories[category] || false;
}

export function hasAnyOptionalConsent(consent: CookieConsentState): boolean {
  return (
    consent.categories.analytics ||
    consent.categories.performance ||
    consent.vendors.posthog ||
    consent.vendors.sentry ||
    consent.vendors.grafana
  );
}

export function persistTrackingConsent(
  consent: CookieConsentState,
  source: string,
  savedAt = new Date(),
): void {
  const storage = getSafeLocalStorage();

  storage.setItem(STORAGE_KEY_V3, JSON.stringify(createStoredConsent(consent, source, savedAt)));

  storage.setItem(STORAGE_KEY_V1, hasAnyOptionalConsent(consent) ? 'accepted' : 'declined');
}

function clearStoredTrackingConsentVersions(): void {
  const storage = getSafeLocalStorage();
  storage.removeItem(STORAGE_KEY_V3);
  storage.removeItem(STORAGE_KEY_V2);
  storage.removeItem(STORAGE_KEY_V1);
}
