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

export function deriveConsentState(consent: CookieConsentState): CookieConsentState {
  return {
    categories: {
      essentials: true,
      analytics: OPTIONAL_ANALYTICS_PURPOSES.some((purpose) => consent.analytics[purpose]),
      performance: consent.vendors.errorReporting || consent.analytics.errorTracking,
    },
    vendors: {
      stripe: true,
      identity: true,
      analytics: hasAnyAnalyticsPurpose(consent.analytics),
      errorReporting: consent.vendors.errorReporting,
    },
    analytics: { ...consent.analytics },
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
  const nextAnalytics = { ...consent.analytics };

  if (category === 'analytics') {
    for (const purpose of OPTIONAL_ANALYTICS_PURPOSES) {
      nextAnalytics[purpose] = nextValue;
    }
  } else {
    nextVendors.errorReporting = nextValue;
    nextAnalytics.errorTracking = nextValue;
  }

  return deriveConsentState({
    categories: { ...consent.categories, [category]: nextValue },
    vendors: nextVendors,
    analytics: nextAnalytics,
  });
}

export function toggleConsentVendor(
  consent: CookieConsentState,
  vendor: keyof CookieConsentState['vendors'],
  category?: keyof CookieConsentState['categories'],
): CookieConsentState {
  if (vendor === 'stripe' || vendor === 'identity') {
    return consent;
  }

  const nextValue = !consent.vendors[vendor];
  const nextVendors = { ...consent.vendors, [vendor]: nextValue };
  const nextAnalytics = { ...consent.analytics };

  if (vendor === 'analytics') {
    for (const purpose of Object.keys(nextAnalytics) as AnalyticsPurpose[]) {
      nextAnalytics[purpose] = nextValue;
    }
  } else if (category === 'performance') {
    nextAnalytics.errorTracking = nextAnalytics.errorTracking && nextValue;
  }

  return deriveConsentState({
    categories: { ...consent.categories },
    vendors: nextVendors,
    analytics: nextAnalytics,
  });
}

export function toggleConsentAnalyticsPurpose(
  consent: CookieConsentState,
  purpose: AnalyticsPurpose,
): CookieConsentState {
  return deriveConsentState({
    categories: { ...consent.categories },
    vendors: { ...consent.vendors },
    analytics: {
      ...consent.analytics,
      [purpose]: !consent.analytics[purpose],
    },
  });
}
