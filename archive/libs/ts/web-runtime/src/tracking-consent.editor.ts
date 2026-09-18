import {
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  type CookieConsentState,
  consentStateFromPurposes,
} from './tracking-consent.storage';

export type AnalyticsPurpose = keyof CookieConsentState['analytics'];

export const OPTIONAL_ANALYTICS_PURPOSES: AnalyticsPurpose[] = ['productAnalytics'];

export function hasAnyAnalyticsPurpose(analytics: CookieConsentState['analytics']): boolean {
  return Object.values(analytics).some((value) => value);
}

export function deriveConsentState(consent: CookieConsentState): CookieConsentState {
  return consentStateFromPurposes(consent.analytics);
}

export function toggleConsentCategory(
  consent: CookieConsentState,
  category: keyof CookieConsentState['categories'],
): CookieConsentState {
  if (category === 'essentials') {
    return consent;
  }

  const nextAnalytics = { ...consent.analytics };
  const nextValue = !consent.categories[category];
  for (const purpose of CATEGORY_ANALYTICS_PURPOSES_MAP[category]) {
    nextAnalytics[purpose] = nextValue;
  }

  return consentStateFromPurposes(nextAnalytics);
}

export function toggleConsentVendor(
  consent: CookieConsentState,
  vendor: keyof CookieConsentState['vendors'],
  category?: keyof CookieConsentState['categories'],
): CookieConsentState {
  void category;

  if (vendor === 'stripe' || vendor === 'identity' || vendor === 'cloudflare') {
    return consent;
  }

  if (vendor === 'posthog') {
    return toggleConsentCategory(consent, 'analytics');
  }

  return toggleConsentCategory(consent, 'performance');
}

export function toggleConsentAnalyticsPurpose(
  consent: CookieConsentState,
  purpose: AnalyticsPurpose,
): CookieConsentState {
  return consentStateFromPurposes({
    ...consent.analytics,
    [purpose]: !consent.analytics[purpose],
  });
}

export function revokeTrackingConsentType(
  consent: CookieConsentState,
  consentType: string,
): CookieConsentState {
  const purpose = Object.entries(ANALYTICS_PURPOSE_CONSENT_TYPES).find(
    ([, mappedConsentType]) => mappedConsentType === consentType,
  )?.[0] as AnalyticsPurpose | undefined;

  if (purpose) {
    return consent.analytics[purpose]
      ? toggleConsentAnalyticsPurpose(consent, purpose)
      : deriveConsentState(consent);
  }

  const revokesAnalytics =
    consentType === 'cookie_consent' ||
    consentType === 'cookie_consent_analytics' ||
    consentType === 'cookie_consent_vendor_analytics' ||
    consentType === 'cookie_consent_vendor_posthog';
  const revokesPerformance =
    consentType === 'cookie_consent' ||
    consentType === 'cookie_consent_performance' ||
    consentType === 'cookie_consent_vendor_error_reporting' ||
    consentType === 'cookie_consent_vendor_sentry' ||
    consentType === 'cookie_consent_vendor_grafana';

  const analytics = { ...consent.analytics };
  if (revokesAnalytics) {
    for (const analyticsPurpose of CATEGORY_ANALYTICS_PURPOSES_MAP.analytics) {
      analytics[analyticsPurpose] = false;
    }
  }
  if (revokesPerformance) {
    analytics.errorTracking = false;
  }

  return consentStateFromPurposes(analytics);
}
