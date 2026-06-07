import type { CookieConsentState } from './tracking-consent';

export type PostHogPurpose = keyof CookieConsentState['posthog'];

export const ANALYTICS_POSTHOG_PURPOSES: PostHogPurpose[] = [
  'productAnalytics',
  'autocaptureHeatmaps',
  'sessionReplay',
  'surveysFeedback',
  'featureFlags',
];

export function hasAnyPostHogPurpose(posthog: CookieConsentState['posthog']): boolean {
  return Object.values(posthog).some((value) => value);
}

export function deriveConsentState(consent: CookieConsentState): CookieConsentState {
  return {
    categories: {
      essentials: true,
      analytics: ANALYTICS_POSTHOG_PURPOSES.some((purpose) => consent.posthog[purpose]),
      performance: consent.vendors.sentry || consent.posthog.errorTracking,
    },
    vendors: {
      stripe: true,
      identity: true,
      posthog: hasAnyPostHogPurpose(consent.posthog),
      sentry: consent.vendors.sentry,
    },
    posthog: { ...consent.posthog },
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
  const nextPosthog = { ...consent.posthog };

  if (category === 'analytics') {
    for (const purpose of ANALYTICS_POSTHOG_PURPOSES) {
      nextPosthog[purpose] = nextValue;
    }
  } else {
    nextVendors.sentry = nextValue;
    nextPosthog.errorTracking = nextValue;
  }

  return deriveConsentState({
    categories: { ...consent.categories, [category]: nextValue },
    vendors: nextVendors,
    posthog: nextPosthog,
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
  const nextPosthog = { ...consent.posthog };

  if (vendor === 'posthog') {
    for (const purpose of Object.keys(nextPosthog) as PostHogPurpose[]) {
      nextPosthog[purpose] = nextValue;
    }
  } else if (category === 'performance') {
    nextPosthog.errorTracking = nextPosthog.errorTracking && nextValue;
  }

  return deriveConsentState({
    categories: { ...consent.categories },
    vendors: nextVendors,
    posthog: nextPosthog,
  });
}

export function toggleConsentPostHogPurpose(
  consent: CookieConsentState,
  purpose: PostHogPurpose,
): CookieConsentState {
  return deriveConsentState({
    categories: { ...consent.categories },
    vendors: { ...consent.vendors },
    posthog: {
      ...consent.posthog,
      [purpose]: !consent.posthog[purpose],
    },
  });
}
