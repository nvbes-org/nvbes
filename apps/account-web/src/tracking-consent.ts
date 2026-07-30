import { accountClient } from '@nvbes/identity-client';
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
    listConsents: (options?: { signal?: AbortSignal }) => accountClient.listConsents(options),
    grantConsent: (consentType: string, documentVersion: string) =>
      accountClient.grantConsent(consentType, documentVersion),
    revokeConsent: (consentType: string, documentVersion: string) =>
      accountClient.revokeConsent(consentType, documentVersion),
    isAuthenticated: async () => {
      try {
        await accountClient.getMe();
        return true;
      } catch {
        return false;
      }
    },
  } satisfies {
    listConsents: (options?: { signal?: AbortSignal }) => Promise<
      Array<{
        consent_type: string;
        document_version: string;
        granted_at: string;
        revoked_at?: string | null;
      }>
    >;
    grantConsent: (consentType: string, documentVersion: string) => Promise<unknown>;
    revokeConsent: (consentType: string, documentVersion: string) => Promise<unknown>;
    isAuthenticated: () => boolean | Promise<boolean>;
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
