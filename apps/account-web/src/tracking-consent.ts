import { accountClient } from './account.client';
import { getAccountAccessToken } from './account.oauth.access-token';
import {
  ACCEPT_ALL_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  type CookieConsentState,
  createTrackingConsentApi,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  revokeTrackingConsentType,
  TRACKING_CONSENT_CHANGED_EVENT,
  type TrackingConsentStoredValue,
} from '@nvbes/web-runtime';

const trackingConsentApi = createTrackingConsentApi({
  identityClient: {
    listConsents: (options?: { signal?: AbortSignal }) =>
      accountClient.listConsents({ signal: options?.signal }).then(({ consents }) => consents),
    grantConsent: (consentType: string, documentVersion: string) =>
      accountClient.grantConsent({
        consent_type: consentType,
        document_version: documentVersion,
      }),
    revokeConsent: (consentType: string, documentVersion: string) =>
      accountClient.revokeConsent({
        consent_type: consentType,
        document_version: documentVersion,
      }),
    isAuthenticated: () => getAccountAccessToken() !== null,
  },
  defaultSource: 'account-web',
});

export {
  ACCEPT_ALL_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
  CATEGORY_ANALYTICS_PURPOSES_MAP,
  CATEGORY_VENDORS_MAP,
  type CookieConsentState,
  DECLINE_ALL_CONSENT,
  DEFAULT_CONSENT,
  revokeTrackingConsentType,
  TRACKING_CONSENT_CHANGED_EVENT,
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
