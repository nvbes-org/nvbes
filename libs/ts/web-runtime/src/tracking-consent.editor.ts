import type { CookieConsentState } from './tracking-consent';

export type AnalyticsPurpose = keyof CookieConsentState['analytics'];

export const OPTIONAL_ANALYTICS_PURPOSES: AnalyticsPurpose[] = [
  'productAnalytics',
  'autocaptureHeatmaps',
  'sessionReplay',
  'surveysFeedback',
  'featureFlags',
];

export function hasAnyAnalyticsPurpose(analytics: CookieConsentState['analytics']): boolean {
  return Object.values(analytics).some((value) => value);
}

function analyticsConsentForState(consent: CookieConsentState): CookieConsentState['analytics'] {
  const analyticsGranted = consent.categories.analytics || consent.vendors.posthog;
  const performanceGranted =
    consent.categories.performance || consent.vendors.sentry || consent.vendors.grafana;

  return {
    productAnalytics: analyticsGranted,
    autocaptureHeatmaps: analyticsGranted,
    sessionReplay: analyticsGranted,
    surveysFeedback: analyticsGranted,
    featureFlags: analyticsGranted,
    errorTracking: performanceGranted,
  };
}

export function deriveConsentState(consent: CookieConsentState): CookieConsentState {
  const vendors = {
    stripe: true,
    identity: true,
    cloudflare: true,
    posthog: consent.vendors.posthog,
    sentry: consent.vendors.sentry,
    grafana: consent.vendors.grafana,
  };
  const categories = {
    essentials: true,
    analytics: consent.categories.analytics || vendors.posthog,
    performance: consent.categories.performance || vendors.sentry || vendors.grafana,
  };

  return {
    categories,
    vendors,
    analytics: analyticsConsentForState({ ...consent, categories, vendors }),
  };
}

export function toggleConsentCategory(
  consent: CookieConsentState,
  category: keyof CookieConsentState['categories'],
): CookieConsentState {
  if (category === 'essentials') {
    return consent;
  }

  const nextValue = !consent.categories[category];
  const nextVendors = { ...consent.vendors };

  if (category === 'analytics') {
    nextVendors.posthog = nextValue;
  } else {
    nextVendors.sentry = nextValue;
    nextVendors.grafana = nextValue;
  }

  return deriveConsentState({
    categories: { ...consent.categories, [category]: nextValue },
    vendors: nextVendors,
    analytics: { ...consent.analytics },
  });
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

  const nextValue = !consent.vendors[vendor];
  const nextVendors = { ...consent.vendors, [vendor]: nextValue };

  return deriveConsentState({
    categories: { ...consent.categories },
    vendors: nextVendors,
    analytics: { ...consent.analytics },
  });
}

export function toggleConsentAnalyticsPurpose(
  consent: CookieConsentState,
  purpose: AnalyticsPurpose,
): CookieConsentState {
  void purpose;
  return deriveConsentState(consent);
}
