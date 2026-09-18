import { type AnalyticsPurposeConsent, EMPTY_ANALYTICS_CONSENT } from './analytics';
import { getSafeLocalStorage } from './safe-storage';
import { z } from 'zod';

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
  version: 4;
  savedAt: string;
  expiresAt: string;
  source: string;
  consent: CookieConsentState;
}

type ConsentCategory = keyof CookieConsentState['categories'];
type ConsentVendor = keyof CookieConsentState['vendors'];
type AnalyticsPurpose = keyof AnalyticsPurposeConsent;

export const TRACKING_CONSENT_CHANGED_EVENT = 'nvbes:tracking-consent-changed';
const CONSENT_TTL_DAYS = 183;
const STORAGE_KEY_V4 = 'nvbes.tracking-consent.v4';
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
  analytics: ['productAnalytics'],
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

function hasProductAnalyticsPurpose(analytics: AnalyticsPurposeConsent): boolean {
  return (
    analytics.productAnalytics ||
    analytics.autocaptureHeatmaps ||
    analytics.sessionReplay ||
    analytics.surveysFeedback ||
    analytics.featureFlags
  );
}

export function consentStateFromPurposes(analytics: AnalyticsPurposeConsent): CookieConsentState {
  const analyticsGranted = hasProductAnalyticsPurpose(analytics);
  const performanceGranted = analytics.errorTracking;

  return {
    categories: {
      essentials: true,
      analytics: analyticsGranted,
      performance: performanceGranted,
    },
    vendors: {
      stripe: true,
      identity: true,
      cloudflare: true,
      posthog: analyticsGranted,
      sentry: performanceGranted,
      grafana: performanceGranted,
    },
    analytics: { ...analytics },
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
  analytics: {
    ...EMPTY_ANALYTICS_CONSENT,
    productAnalytics: true,
    errorTracking: true,
  },
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
    version: 4,
    savedAt: savedAt.toISOString(),
    expiresAt: consentExpiry(savedAt),
    source,
    consent: cloneConsent(consent),
  };
}

const storedConsentSchema = z.object({
  version: z.literal(4),
  savedAt: z.string().datetime(),
  expiresAt: z.string().datetime(),
  source: z.string().trim().min(1),
  consent: z.object({
    analytics: z.object({
      productAnalytics: z.boolean(),
      autocaptureHeatmaps: z.boolean(),
      sessionReplay: z.boolean(),
      surveysFeedback: z.boolean(),
      errorTracking: z.boolean(),
      featureFlags: z.boolean(),
    }),
  }),
});

export function readTrackingConsentStoredValue(): TrackingConsentStoredValue | null {
  const storage = getSafeLocalStorage();
  const value = storage.getItem(STORAGE_KEY_V4);
  if (!value) {
    return null;
  }

  try {
    const parsed = storedConsentSchema.parse(JSON.parse(value));
    const expiresAt = Date.parse(parsed.expiresAt);
    const savedAt = Date.parse(parsed.savedAt);
    if (
      expiresAt <= Date.now() ||
      expiresAt <= savedAt ||
      expiresAt > Date.parse(consentExpiry(new Date(savedAt)))
    ) {
      clearStoredTrackingConsentVersions();
      return null;
    }

    return { ...parsed, consent: consentStateFromPurposes(parsed.consent.analytics) };
  } catch {
    clearStoredTrackingConsentVersions();
    return null;
  }
}

export function getTrackingConsent(): CookieConsentState | null {
  return readTrackingConsentStoredValue()?.consent ?? null;
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
  return Object.values(consent.analytics).some(Boolean);
}

export function persistTrackingConsent(
  consent: CookieConsentState,
  source: string,
  savedAt = new Date(),
): void {
  const storage = getSafeLocalStorage();

  storage.setItem(STORAGE_KEY_V4, JSON.stringify(createStoredConsent(consent, source, savedAt)));
}

function clearStoredTrackingConsentVersions(): void {
  const storage = getSafeLocalStorage();
  storage.removeItem(STORAGE_KEY_V4);
  storage.removeItem(STORAGE_KEY_V3);
  storage.removeItem(STORAGE_KEY_V2);
  storage.removeItem(STORAGE_KEY_V1);
}
