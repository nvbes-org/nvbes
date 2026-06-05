import posthog from 'posthog-js';
import { isVendorAccepted } from './tracking-consent';

let initialized = false;

export function initPostHog() {
  if (initialized || !isVendorAccepted('posthog')) {
    return;
  }

  const apiKey = import.meta.env.VITE_POSTHOG_KEY;
  const apiHost = import.meta.env.VITE_POSTHOG_HOST || 'https://app.posthog.com';

  if (!apiKey) {
    console.warn('PostHog API Key not found, skipping initialization');
    return;
  }

  posthog.init(apiKey, {
    api_host: apiHost,
    person_profiles: 'identified_only',
    capture_pageview: true,
  });

  initialized = true;
}

export const trackEvent = (name: string, properties?: Record<string, unknown>) => {
  if (!isVendorAccepted('posthog')) {
    return;
  }

  posthog.capture(name, properties);
};

export const identifyUser = (userId: string, traits?: Record<string, unknown>) => {
  if (!isVendorAccepted('posthog')) {
    return;
  }

  posthog.identify(userId, traits);
};
