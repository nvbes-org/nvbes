import {
  EMPTY_ANALYTICS_CONSENT,
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
  type AnalyticsPurposeConsent,
} from '@nvbes/web-runtime/analytics';
import {
  TRACKING_CONSENT_CHANGED_EVENT,
  type CookieConsentState,
  getAnalyticsConsent,
} from './tracking-consent';

let initialized = false;
let pageTrackingInstalled = false;
let lastTrackedPath: string | null = null;

function currentPath(): string {
  if (typeof window === 'undefined') {
    return '/';
  }
  return window.location.pathname || '/';
}

function subscribeToConsent(listener: (consent: AnalyticsPurposeConsent) => void): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }

  const handleConsentChange = (event: Event) => {
    if (!(event instanceof CustomEvent)) {
      return;
    }

    const detail = event.detail as { consent?: CookieConsentState };
    listener(detail.consent?.analytics ?? EMPTY_ANALYTICS_CONSENT);
  };

  window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
  return () => window.removeEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
}

function captureCurrentPageView(): void {
  const routePath = currentPath();
  if (routePath === lastTrackedPath) {
    return;
  }

  lastTrackedPath = routePath;
  void trackProductEvent('marketing.page_viewed', {
    event_source: 'router',
    source: 'account-web',
  });
}

function schedulePageView(): void {
  window.requestAnimationFrame(captureCurrentPageView);
}

function installPageTracking(): void {
  if (pageTrackingInstalled || typeof window === 'undefined') {
    return;
  }

  const originalPushState: History['pushState'] = window.history.pushState.bind(window.history);
  const originalReplaceState: History['replaceState'] = window.history.replaceState.bind(
    window.history,
  );

  window.history.pushState = function pushStateWithAnalyticsTracking(
    ...args: Parameters<History['pushState']>
  ) {
    const result = originalPushState(...args);
    schedulePageView();
    return result;
  };

  window.history.replaceState = function replaceStateWithAnalyticsTracking(
    ...args: Parameters<History['replaceState']>
  ) {
    const result = originalReplaceState(...args);
    schedulePageView();
    return result;
  };

  window.addEventListener('popstate', schedulePageView);
  pageTrackingInstalled = true;
}

export function initAnalytics() {
  if (initialized) {
    return;
  }

  initialized = true;
  installPageTracking();

  initAnalyticsRuntime({
    appName: 'account-web',
    analyticsSalt: import.meta.env.VITE_ANALYTICS_ID_SALT,
    getConsent: getAnalyticsConsent,
    onConsentChange: subscribeToConsent,
    getRoutePath: currentPath,
    getCommonProperties: () => ({
      app_name: 'account-web',
      event_source: 'browser',
    }),
    transport: createBrowserAnalyticsTransport({
      appName: 'account-web',
      environment: import.meta.env.MODE,
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
