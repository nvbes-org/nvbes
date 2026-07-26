import type { UserConsent } from '@nvbes/identity-client';
import { describe, expect, it } from 'vite-plus/test';
import { createPrivacyOverview } from '../src/pages/AccountPrivacyPage.presentation';
import { ACCEPT_ALL_CONSENT, DECLINE_ALL_CONSENT } from '../src/tracking-consent';

function consent(consentType: string): UserConsent {
  return {
    id: `${consentType}-id`,
    principal_id: 'principal-id',
    consent_type: consentType,
    document_version: 'v1',
    ip_address: null,
    granted_at: '2026-07-20T10:00:00Z',
    revoked_at: null,
  };
}

describe('privacy page presentation', () => {
  it('summarizes only understandable active choices', () => {
    const overview = createPrivacyOverview({
      consents: [
        consent('privacy_policy'),
        consent('analytics_product_analytics'),
        consent('cookie_consent_vendor_posthog'),
      ],
      trackingConsent: ACCEPT_ALL_CONSENT,
      gpcEnabled: true,
    });

    expect(overview.optionalEnabledCount).toBe(2);
    expect(overview.recordedChoiceCount).toBe(2);
    expect(overview.gpcProtected).toBe(true);
  });

  it('reports a privacy-first default when optional purposes are disabled', () => {
    const overview = createPrivacyOverview({
      consents: [],
      trackingConsent: DECLINE_ALL_CONSENT,
      gpcEnabled: false,
    });

    expect(overview.optionalEnabledCount).toBe(0);
    expect(overview.optionalTotal).toBe(2);
    expect(overview.recordedChoiceCount).toBe(0);
  });
});
