import {
  capturePostHogException,
  getFeatureFlag,
  getFeatureFlagPayload,
  identifyProductUser,
  initPostHogRuntime,
  isFeatureEnabled,
  setPostHogWorkspaceGroup,
  startPrivacySafeReplay,
  stopPrivacySafeReplay,
  trackExperimentExposure,
  trackProductEvent,
} from '@nvbes/web-runtime/posthog';
import { getPostHogConsent } from './tracking-consent';
import {
  blockedRoutePatterns,
  captureCurrentPageView,
  currentPath,
  installPageTracking,
} from './drive.posthog.pageview';
import { subscribeToConsent } from './drive.posthog.consent';

let initialized = false;

export function initPostHog() {
  if (initialized) {
    return;
  }

  initialized = true;
  installPageTracking();

  initPostHogRuntime({
    appName: 'drive-web',
    apiKey: import.meta.env.VITE_POSTHOG_KEY,
    apiHost: import.meta.env.VITE_POSTHOG_HOST || 'https://eu.i.posthog.com',
    analyticsSalt:
      import.meta.env.VITE_ANALYTICS_ID_SALT || import.meta.env.VITE_POSTHOG_ANALYTICS_SALT,
    getConsent: getPostHogConsent,
    onConsentChange: subscribeToConsent,
    getRoutePath: currentPath,
    getCommonProperties: () => ({
      app_name: 'drive-web',
      event_source: 'browser',
    }),
    blockedRoutePatterns,
  });

  captureCurrentPageView();
}

export const trackEvent = (name: string, properties?: Record<string, unknown>) => {
  void trackProductEvent(name, properties);
};

export const identifyUser = (userId: string, traits?: Record<string, unknown>) => {
  void identifyProductUser(userId, traits);
};

export {
  capturePostHogException,
  getFeatureFlag,
  getFeatureFlagPayload,
  isFeatureEnabled,
  setPostHogWorkspaceGroup,
  startPrivacySafeReplay,
  stopPrivacySafeReplay,
  trackExperimentExposure,
};
