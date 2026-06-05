import posthog from 'posthog-js/dist/module.no-external';
import {
  TRACKING_CONSENT_CHANGED_EVENT,
  type CookieConsentState,
  isVendorAccepted,
} from './tracking-consent';

let initialized = false;
let consentListenerInstalled = false;

function currentHostCookieDomains(): string[] {
  if (typeof window === 'undefined') {
    return [];
  }

  const host = window.location.hostname;
  const parts = host.split('.').filter(Boolean);
  if (parts.length < 2 || host === 'localhost' || /^[\d.]+$/.test(host)) {
    return [host];
  }

  return [host, `.${parts.slice(-2).join('.')}`];
}

function clearPostHogStorage() {
  if (typeof window === 'undefined') {
    return;
  }

  for (const storage of [window.localStorage, window.sessionStorage]) {
    for (const key of Object.keys(storage)) {
      const normalized = key.toLowerCase();
      if (normalized.startsWith('ph_') || normalized.includes('posthog')) {
        storage.removeItem(key);
      }
    }
  }

  for (const rawCookie of document.cookie.split(';')) {
    const cookieName = rawCookie.split('=')[0]?.trim();
    if (!cookieName) continue;

    const normalized = cookieName.toLowerCase();
    if (!normalized.startsWith('ph_') && !normalized.includes('posthog')) continue;

    document.cookie = `${cookieName}=; Max-Age=0; path=/; SameSite=Lax`;
    for (const domain of currentHostCookieDomains()) {
      document.cookie = `${cookieName}=; Max-Age=0; path=/; domain=${domain}; SameSite=Lax`;
    }
  }
}

function enablePostHogCapture() {
  posthog.opt_in_capturing();
  posthog.capture('$pageview');
}

function disablePostHogCapture() {
  if (!initialized) {
    clearPostHogStorage();
    return;
  }

  posthog.stopSessionRecording();
  posthog.opt_out_capturing();
  posthog.reset();
  clearPostHogStorage();
}

function installConsentListener() {
  if (consentListenerInstalled || typeof window === 'undefined') {
    return;
  }

  window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, (event) => {
    if (!(event instanceof CustomEvent)) {
      return;
    }

    const detail = event.detail as { consent?: CookieConsentState };
    if (detail.consent?.vendors.posthog) {
      const wasInitialized = initialized;
      initPostHog();
      if (wasInitialized && initialized) {
        enablePostHogCapture();
      }
    } else {
      disablePostHogCapture();
    }
  });

  consentListenerInstalled = true;
}

export function initPostHog() {
  installConsentListener();

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
    autocapture: false,
    capture_pageview: false,
    disable_session_recording: true,
    opt_out_capturing_by_default: true,
    person_profiles: 'identified_only',
    persistence: 'localStorage',
  });

  initialized = true;
  enablePostHogCapture();
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
