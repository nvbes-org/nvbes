import { describe, expect, it } from 'vite-plus/test';
import {
  revokeTrackingConsentType,
  toggleConsentAnalyticsPurpose,
  toggleConsentCategory,
} from './tracking-consent.editor';
import { DEFAULT_CONSENT } from './tracking-consent.storage';

describe('tracking consent editor', () => {
  it('enables one analytics purpose without enabling the others', () => {
    const consent = toggleConsentAnalyticsPurpose(DEFAULT_CONSENT, 'productAnalytics');

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
    const consent = toggleConsentCategory(DEFAULT_CONSENT, 'analytics');

    expect(consent.analytics.productAnalytics).toBe(true);
    expect(consent.analytics.autocaptureHeatmaps).toBe(false);
    expect(consent.analytics.sessionReplay).toBe(false);
    expect(consent.analytics.surveysFeedback).toBe(false);
    expect(consent.analytics.featureFlags).toBe(false);
    expect(consent.analytics.errorTracking).toBe(false);
  });

  it('revokes one backend purpose without revoking the other purposes', () => {
    const withPurposes = toggleConsentAnalyticsPurpose(
      toggleConsentAnalyticsPurpose(
        toggleConsentAnalyticsPurpose(DEFAULT_CONSENT, 'productAnalytics'),
        'sessionReplay',
      ),
      'featureFlags',
    );
    const consent = revokeTrackingConsentType(withPurposes, 'analytics_session_replay');

    expect(consent.analytics.productAnalytics).toBe(true);
    expect(consent.analytics.sessionReplay).toBe(false);
    expect(consent.analytics.featureFlags).toBe(true);
  });
});
