import {
  captureAnalyticsException,
  createBrowserAnalyticsTransport,
  getFeatureFlag,
  getFeatureFlagPayload,
  identifyProductUser,
  initAnalyticsRuntime,
  isFeatureEnabled,
  setAnalyticsWorkspaceGroup,
  startPrivacySafeReplay,
  stopPrivacySafeReplay,
  trackExperimentExposure,
  trackProductEvent,
} from '@nvbes/web-runtime/analytics';
import { getAnalyticsConsent } from './tracking-consent';
import {
  blockedRoutePatterns,
  captureCurrentPageView,
  currentPath,
  installPageTracking,
} from './drive.analytics.pageview';
import { subscribeToConsent } from './drive.analytics.consent';

let initialized = false;

export function initAnalytics() {
  if (initialized) {
    return;
  }

  initialized = true;
  installPageTracking();

  initAnalyticsRuntime({
    appName: 'cloud-web',
    analyticsSalt: import.meta.env.VITE_ANALYTICS_ID_SALT,
    getConsent: getAnalyticsConsent,
    onConsentChange: subscribeToConsent,
    getRoutePath: currentPath,
    getCommonProperties: () => ({
      app_name: 'cloud-web',
      event_source: 'browser',
    }),
    blockedRoutePatterns,
    transport: createBrowserAnalyticsTransport({
      appName: 'cloud-web',
      environment: import.meta.env.MODE,
      sentryDsn: import.meta.env.VITE_SENTRY_DSN,
      sentryTracesSampleRate: Number(import.meta.env.VITE_SENTRY_TRACES_SAMPLE_RATE || 0),
      posthogKey: import.meta.env.VITE_POSTHOG_KEY,
      posthogHost: import.meta.env.VITE_POSTHOG_HOST,
    }),
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
  captureAnalyticsException,
  getFeatureFlag,
  getFeatureFlagPayload,
  isFeatureEnabled,
  setAnalyticsWorkspaceGroup,
  startPrivacySafeReplay,
  stopPrivacySafeReplay,
  trackExperimentExposure,
};
