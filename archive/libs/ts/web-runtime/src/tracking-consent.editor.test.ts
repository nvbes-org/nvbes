import { expect, it } from 'vite-plus/test';
import {
  ACCEPT_ALL_CONSENT,
  DECLINE_ALL_CONSENT,
  ANALYTICS_PURPOSE_CONSENT_TYPES,
} from './tracking-consent.storage';
import {
  hasAnyAnalyticsPurpose,
  revokeTrackingConsentType,
  toggleConsentVendor,
  toggleConsentCategory,
  toggleConsentAnalyticsPurpose,
} from './tracking-consent.editor';

it('enables one analytics purpose without enabling the others', () => {
  const consent = toggleConsentAnalyticsPurpose(DECLINE_ALL_CONSENT, 'productAnalytics');
  expect(consent.categories.analytics).toBe(true);
  expect(consent.vendors.posthog).toBe(true);
  expect(consent.analytics).toEqual({
    productAnalytics: true,
    autocaptureHeatmaps: false,
    sessionReplay: false,
    surveysFeedback: false,
    errorTracking: false,
    featureFlags: false,
  });
});
it('uses a category toggle as a group command for its purposes', () => {
  const consent = toggleConsentCategory(DECLINE_ALL_CONSENT, 'analytics');
  expect(consent.analytics).toEqual({
    productAnalytics: true,
    autocaptureHeatmaps: false,
    sessionReplay: false,
    surveysFeedback: false,
    featureFlags: false,
    errorTracking: false,
  });
});
it('revokes one backend purpose without revoking the other purposes', () => {
  const withPurposes = toggleConsentAnalyticsPurpose(
    toggleConsentAnalyticsPurpose(
      toggleConsentAnalyticsPurpose(DECLINE_ALL_CONSENT, 'productAnalytics'),
      'sessionReplay',
    ),
    'featureFlags',
  );
  const consent = revokeTrackingConsentType(withPurposes, 'analytics_session_replay');
  expect(consent.analytics.productAnalytics).toBe(true);
  expect(consent.analytics.sessionReplay).toBe(false);
  expect(consent.analytics.featureFlags).toBe(true);
});

it('distinguishes absent optional consent from explicit consent', () => {
  expect(hasAnyAnalyticsPurpose(DECLINE_ALL_CONSENT.analytics)).toBe(false);
  expect(hasAnyAnalyticsPurpose(ACCEPT_ALL_CONSENT.analytics)).toBe(true);
  expect(toggleConsentCategory(ACCEPT_ALL_CONSENT, 'essentials')).toBe(ACCEPT_ALL_CONSENT);
});
it.each(['stripe', 'identity', 'cloudflare'] as const)(
  'does not toggle essential vendor %s',
  (vendor) => {
    expect(toggleConsentVendor(DECLINE_ALL_CONSENT, vendor)).toBe(DECLINE_ALL_CONSENT);
  },
);
it.each(['posthog', 'sentry', 'grafana'] as const)(
  'toggles the corresponding optional category for %s',
  (vendor) => {
    const category = vendor === 'posthog' ? 'analytics' : 'performance';
    const enabled = toggleConsentVendor(DECLINE_ALL_CONSENT, vendor);
    expect(enabled.categories[category]).toBe(true);
    expect(toggleConsentVendor(enabled, vendor)).toEqual(DECLINE_ALL_CONSENT);
  },
);
it.each(Object.entries(ANALYTICS_PURPOSE_CONSENT_TYPES))(
  'revokes purpose %s idempotently',
  (purpose, consentType) => {
    const result = revokeTrackingConsentType(ACCEPT_ALL_CONSENT, consentType);
    expect(result.analytics).toEqual({ ...ACCEPT_ALL_CONSENT.analytics, [purpose]: false });
    expect(revokeTrackingConsentType(result, consentType)).toEqual(result);
  },
);
it.each([
  'cookie_consent_analytics',
  'cookie_consent_vendor_analytics',
  'cookie_consent_vendor_posthog',
])('revokes analytics via legacy type %s without disabling errors', (consentType) => {
  expect(revokeTrackingConsentType(ACCEPT_ALL_CONSENT, consentType).analytics).toEqual({
    ...DECLINE_ALL_CONSENT.analytics,
    errorTracking: true,
  });
});
it.each([
  'cookie_consent_performance',
  'cookie_consent_vendor_error_reporting',
  'cookie_consent_vendor_sentry',
  'cookie_consent_vendor_grafana',
])('revokes errors via type %s without disabling product analytics', (consentType) => {
  expect(revokeTrackingConsentType(ACCEPT_ALL_CONSENT, consentType).analytics).toEqual({
    ...ACCEPT_ALL_CONSENT.analytics,
    errorTracking: false,
  });
});
it('revokes all optional tracking but preserves essentials and ignores unknown types', () => {
  expect(revokeTrackingConsentType(ACCEPT_ALL_CONSENT, 'cookie_consent')).toEqual(
    DECLINE_ALL_CONSENT,
  );
  expect(revokeTrackingConsentType(ACCEPT_ALL_CONSENT, 'unknown')).toEqual(ACCEPT_ALL_CONSENT);
});
