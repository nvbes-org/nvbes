import {
  EMPTY_POSTHOG_CONSENT,
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
  type PostHogPurposeConsent,
} from '@nvbes/web-runtime/posthog';
import {
  TRACKING_CONSENT_CHANGED_EVENT,
  type CookieConsentState,
  getPostHogConsent,
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

function subscribeToConsent(listener: (consent: PostHogPurposeConsent) => void): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }

  const handleConsentChange = (event: Event) => {
    if (!(event instanceof CustomEvent)) {
      return;
    }

    const detail = event.detail as { consent?: CookieConsentState };
    listener(detail.consent?.posthog ?? EMPTY_POSTHOG_CONSENT);
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
    source: 'identity-web',
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

  window.history.pushState = function pushStateWithPostHogTracking(
    ...args: Parameters<History['pushState']>
  ) {
    const result = originalPushState(...args);
    schedulePageView();
    return result;
  };

  window.history.replaceState = function replaceStateWithPostHogTracking(
    ...args: Parameters<History['replaceState']>
  ) {
    const result = originalReplaceState(...args);
    schedulePageView();
    return result;
  };

  window.addEventListener('popstate', schedulePageView);
  pageTrackingInstalled = true;
}

export function initPostHog() {
  if (initialized) {
    return;
  }

  initialized = true;
  installPageTracking();

  initPostHogRuntime({
    appName: 'identity-web',
    apiKey: import.meta.env.VITE_POSTHOG_KEY,
    apiHost: import.meta.env.VITE_POSTHOG_HOST || 'https://eu.i.posthog.com',
    analyticsSalt:
      import.meta.env.VITE_ANALYTICS_ID_SALT || import.meta.env.VITE_POSTHOG_ANALYTICS_SALT,
    getConsent: getPostHogConsent,
    onConsentChange: subscribeToConsent,
    getRoutePath: currentPath,
    getCommonProperties: () => ({
      app_name: 'identity-web',
      event_source: 'browser',
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
  capturePostHogException,
  getFeatureFlag,
  getFeatureFlagPayload,
  isFeatureEnabled,
  setPostHogWorkspaceGroup,
  startPrivacySafeReplay,
  stopPrivacySafeReplay,
  trackExperimentExposure,
};
