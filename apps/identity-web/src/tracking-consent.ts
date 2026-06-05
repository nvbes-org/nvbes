import { identityClient } from '@nvbes/identity-client';

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
}

export interface TrackingConsentStoredValue {
  version: 2;
  savedAt: string;
  expiresAt: string;
  source: string;
  consent: CookieConsentState;
}

export const TRACKING_CONSENT_CHANGED_EVENT = 'nvbes:tracking-consent-changed';
const CONSENT_TTL_DAYS = 183;

export const CATEGORY_VENDORS_MAP = {
  essentials: ['stripe', 'identity'],
  analytics: ['posthog'],
  performance: ['sentry'],
} as const;

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
};

const STORAGE_KEY_V2 = 'nvbes.tracking-consent.v2';
const STORAGE_KEY_V1 = 'nvbes.tracking-consent.v1';

function consentExpiry(savedAt: Date): string {
  const expiresAt = new Date(savedAt);
  expiresAt.setDate(expiresAt.getDate() + CONSENT_TTL_DAYS);
  return expiresAt.toISOString();
}

function createStoredConsent(
  consent: CookieConsentState,
  source: string,
  savedAt = new Date(),
): TrackingConsentStoredValue {
  return {
    version: 2,
    savedAt: savedAt.toISOString(),
    expiresAt: consentExpiry(savedAt),
    source,
    consent,
  };
}

function parseStoredConsent(value: string): CookieConsentState | null {
  const parsed = JSON.parse(value) as CookieConsentState | TrackingConsentStoredValue;
  const consent = 'consent' in parsed ? parsed.consent : parsed;
  const expiresAt = 'expiresAt' in parsed ? Date.parse(parsed.expiresAt) : Number.NaN;

  if (Number.isFinite(expiresAt) && expiresAt <= Date.now()) {
    window.localStorage.removeItem(STORAGE_KEY_V2);
    window.localStorage.removeItem(STORAGE_KEY_V1);
    return null;
  }

  if ('marketing' in consent.categories || 'marketingVendor' in consent.vendors) {
    return {
      categories: {
        essentials: consent.categories.essentials,
        analytics: consent.categories.analytics,
        performance: consent.categories.performance,
      },
      vendors: {
        stripe: consent.vendors.stripe,
        identity: consent.vendors.identity,
        posthog: consent.vendors.posthog,
        sentry: consent.vendors.sentry,
      },
    };
  }

  return consent;
}

export function getTrackingConsent(): CookieConsentState | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const valueV2 = window.localStorage.getItem(STORAGE_KEY_V2);
  if (valueV2) {
    try {
      return parseStoredConsent(valueV2);
    } catch {
      // Corrupted storage, fallback
    }
  }

  // Fallback to V1
  const valueV1 = window.localStorage.getItem(STORAGE_KEY_V1);
  if (valueV1 === 'accepted') {
    window.localStorage.setItem(
      STORAGE_KEY_V2,
      JSON.stringify(createStoredConsent(ACCEPT_ALL_CONSENT, 'legacy-v1-migration')),
    );
    return ACCEPT_ALL_CONSENT;
  } else if (valueV1 === 'declined') {
    window.localStorage.setItem(
      STORAGE_KEY_V2,
      JSON.stringify(createStoredConsent(DECLINE_ALL_CONSENT, 'legacy-v1-migration')),
    );
    return DECLINE_ALL_CONSENT;
  }

  return null;
}

export function isVendorAccepted(vendor: keyof CookieConsentState['vendors']): boolean {
  const consent = getTrackingConsent();
  if (!consent) return false;
  return consent.vendors[vendor] || false;
}

export function isCategoryAccepted(category: keyof CookieConsentState['categories']): boolean {
  const consent = getTrackingConsent();
  if (!consent) return false;
  return consent.categories[category] || false;
}

export function setTrackingConsent(consent: CookieConsentState, source = 'identity-web') {
  window.localStorage.setItem(STORAGE_KEY_V2, JSON.stringify(createStoredConsent(consent, source)));

  // Write v1 for backward compatibility
  const hasAnyOptional = consent.categories.analytics || consent.categories.performance;
  window.localStorage.setItem(STORAGE_KEY_V1, hasAnyOptional ? 'accepted' : 'declined');

  window.dispatchEvent(
    new CustomEvent(TRACKING_CONSENT_CHANGED_EVENT, {
      detail: { consent, source },
    }),
  );

  void (async () => {
    try {
      await syncWithBackend(consent);
    } catch {
      // Ignore if not logged in or endpoint fails
    }
  })();
}

async function syncWithBackend(consent: CookieConsentState) {
  try {
    const consents = await identityClient.listConsents();
    const activeConsents = new Set(
      consents.filter((c) => !c.revoked_at).map((c) => c.consent_type),
    );

    const categoryMapping: Record<keyof CookieConsentState['categories'], string> = {
      essentials: 'cookie_consent_essentials',
      analytics: 'cookie_consent_analytics',
      performance: 'cookie_consent_performance',
    };

    const vendorMapping: Record<keyof CookieConsentState['vendors'], string> = {
      stripe: 'cookie_consent_vendor_stripe',
      identity: 'cookie_consent_vendor_identity',
      posthog: 'cookie_consent_vendor_posthog',
      sentry: 'cookie_consent_vendor_sentry',
    };

    const promises: Promise<unknown>[] = [];

    for (const [cat, consentType] of Object.entries(categoryMapping)) {
      const isGranted = consent.categories[cat as keyof CookieConsentState['categories']];
      const hasOnBackend = activeConsents.has(consentType);
      if (isGranted && !hasOnBackend) {
        promises.push(identityClient.grantConsent(consentType, 'v2'));
      } else if (!isGranted && hasOnBackend) {
        promises.push(identityClient.revokeConsent(consentType, 'v2'));
      }
    }

    for (const [ven, consentType] of Object.entries(vendorMapping)) {
      const isGranted = consent.vendors[ven as keyof CookieConsentState['vendors']];
      const hasOnBackend = activeConsents.has(consentType);
      if (isGranted && !hasOnBackend) {
        promises.push(identityClient.grantConsent(consentType, 'v2'));
      } else if (!isGranted && hasOnBackend) {
        promises.push(identityClient.revokeConsent(consentType, 'v2'));
      }
    }

    // Keep overall 'cookie_consent' active if at least one optional is accepted
    const hasAnyOptional = consent.categories.analytics || consent.categories.performance;
    const hasGeneralBackend = activeConsents.has('cookie_consent');
    if (hasAnyOptional && !hasGeneralBackend) {
      promises.push(identityClient.grantConsent('cookie_consent', 'v2'));
    } else if (!hasAnyOptional && hasGeneralBackend) {
      promises.push(identityClient.revokeConsent('cookie_consent', 'v2'));
    }

    if (promises.length > 0) {
      await Promise.all(promises);
    }
  } catch {
    // Suppress errors during client updates (e.g. offline, unauthenticated)
  }
}

export async function syncTrackingConsent() {
  const current = getTrackingConsent();
  if (current) {
    await syncWithBackend(current);
  }
}
