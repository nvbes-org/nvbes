import {
  captureAnalyticsException,
  createBrowserAnalyticsTransportFromEnv,
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
    transport: createBrowserAnalyticsTransportFromEnv({
      appName: 'cloud-web',
      environment: import.meta.env.MODE,
      env: import.meta.env,
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
