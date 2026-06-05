import { identityClient } from './drive.session';

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

export function getTrackingConsent(): CookieConsentState | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const valueV2 = window.localStorage.getItem(STORAGE_KEY_V2);
  if (valueV2) {
    try {
      const parsed = JSON.parse(valueV2) as CookieConsentState;
      // Clean up potentially loaded marketing keys if parsed
      if ('marketing' in parsed.categories || 'marketingVendor' in parsed.vendors) {
        return {
          categories: {
            essentials: parsed.categories.essentials,
            analytics: parsed.categories.analytics,
            performance: parsed.categories.performance,
          },
          vendors: {
            stripe: parsed.vendors.stripe,
            identity: parsed.vendors.identity,
            posthog: parsed.vendors.posthog,
            sentry: parsed.vendors.sentry,
          },
        };
      }
      return parsed;
    } catch {
      // Corrupted storage, fallback
    }
  }

  // Fallback to V1
  const valueV1 = window.localStorage.getItem(STORAGE_KEY_V1);
  if (valueV1 === 'accepted') {
    window.localStorage.setItem(STORAGE_KEY_V2, JSON.stringify(ACCEPT_ALL_CONSENT));
    return ACCEPT_ALL_CONSENT;
  } else if (valueV1 === 'declined') {
    window.localStorage.setItem(STORAGE_KEY_V2, JSON.stringify(DECLINE_ALL_CONSENT));
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

export function setTrackingConsent(consent: CookieConsentState) {
  window.localStorage.setItem(STORAGE_KEY_V2, JSON.stringify(consent));

  // Write v1 for backward compatibility
  const hasAnyOptional = consent.categories.analytics || consent.categories.performance;
  window.localStorage.setItem(STORAGE_KEY_V1, hasAnyOptional ? 'accepted' : 'declined');

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
    if (typeof identityClient.isAuthenticated === 'function' && !identityClient.isAuthenticated()) {
      return;
    }

    const consents = (await identityClient.listConsents()) as Array<{
      consent_type: string;
      revoked_at?: string | null;
    }>;
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
