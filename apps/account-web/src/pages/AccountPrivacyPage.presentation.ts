import type { UserConsent } from '@nvbes/identity-client';
import type { CookieConsentState } from '../tracking-consent';
import { isVisibleConsentType } from './AccountPrivacyPage.consents';

export type PrivacyOverview = {
  optionalEnabledCount: number;
  optionalTotal: number;
  recordedChoiceCount: number;
  gpcProtected: boolean;
};

export function createPrivacyOverview({
  consents,
  trackingConsent,
  gpcEnabled,
}: {
  consents: UserConsent[];
  trackingConsent: CookieConsentState;
  gpcEnabled: boolean;
}): PrivacyOverview {
  return {
    optionalEnabledCount: [
      trackingConsent.analytics.productAnalytics,
      trackingConsent.analytics.errorTracking,
    ].filter(Boolean).length,
    optionalTotal: 2,
    recordedChoiceCount: consents.filter((consent) => isVisibleConsentType(consent.consent_type))
      .length,
    gpcProtected: gpcEnabled,
  };
}
