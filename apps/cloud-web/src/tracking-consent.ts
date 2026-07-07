import {
  ACCEPT_ALL_CONSENT,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  createTrackingConsentApi,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
  type CookieConsentState,
  type TrackingConsentClient,
  type TrackingConsentStoredValue,
} from '@nvbes/web-runtime';
import { identityClient } from './drive.session';

function asBackendConsents(
  consents: unknown[],
): Awaited<ReturnType<TrackingConsentClient['listConsents']>> {
  return consents as Awaited<ReturnType<TrackingConsentClient['listConsents']>>;
}

const trackingConsentApi = createTrackingConsentApi({
  identityClient: {
    listConsents: async (options?: { signal?: AbortSignal }) => {
      void options;
      return asBackendConsents(await identityClient.listConsents());
    },
    grantConsent: (consentType, documentVersion) =>
      identityClient.grantConsent(consentType, documentVersion),
    revokeConsent: (consentType, documentVersion) =>
      identityClient.revokeConsent(consentType, documentVersion),
    isAuthenticated: () => identityClient.isAuthenticated(),
  } satisfies TrackingConsentClient,
  defaultSource: 'cloud-web',
});

export {
  ACCEPT_ALL_CONSENT,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  TRACKING_CONSENT_CHANGED_EVENT,
  type CookieConsentState,
  type TrackingConsentStoredValue,
};

export const {
  getTrackingConsent,
  getAnalyticsConsent,
  isAnalyticsPurposeAccepted,
  isVendorAccepted,
  isCategoryAccepted,
  setTrackingConsent,
  syncTrackingConsent,
} = trackingConsentApi;
