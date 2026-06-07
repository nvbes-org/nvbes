import { useState } from 'react';
import {
  deriveConsentState,
  toggleConsentCategory,
  toggleConsentPostHogPurpose,
  toggleConsentVendor,
  type PostHogPurpose,
} from '@nvbes/web-runtime';
import {
  DEFAULT_CONSENT,
  getTrackingConsent,
  setTrackingConsent,
  type CookieConsentState,
} from '../tracking-consent';
import {
  CookieConsentAnalyticsSection,
  CookieConsentEssentialSection,
  CookieConsentPerformanceSection,
} from './CookieConsentSettings.sections';
import { getConsentVendorCategory } from './CookieConsentSettings.shared';

export function CookieConsentSettings() {
  const [cookieConsent, setCookieConsent] = useState<CookieConsentState>(
    () => getTrackingConsent() || DEFAULT_CONSENT,
  );

  const persistConsent = (next: CookieConsentState, source: string) => {
    const derived = deriveConsentState(next);
    setCookieConsent(derived);
    setTrackingConsent(derived, source);
  };

  const toggleCategory = (category: keyof CookieConsentState['categories']) => {
    persistConsent(
      toggleConsentCategory(cookieConsent, category),
      `identity-web:account-privacy:${category}`,
    );
  };

  const toggleVendor = (vendor: keyof CookieConsentState['vendors']) => {
    persistConsent(
      toggleConsentVendor(cookieConsent, vendor, getConsentVendorCategory(vendor)),
      `identity-web:account-privacy:${vendor}`,
    );
  };

  const togglePostHogPurpose = (purpose: PostHogPurpose) => {
    persistConsent(
      toggleConsentPostHogPurpose(cookieConsent, purpose),
      `identity-web:account-privacy:posthog:${purpose}`,
    );
  };

  return (
    <div className="space-y-4">
      <CookieConsentEssentialSection />
      <CookieConsentAnalyticsSection
        cookieConsent={cookieConsent}
        onToggleCategory={toggleCategory}
        onToggleVendor={toggleVendor}
        onTogglePostHogPurpose={togglePostHogPurpose}
      />
      <CookieConsentPerformanceSection
        cookieConsent={cookieConsent}
        onToggleCategory={toggleCategory}
        onToggleVendor={toggleVendor}
        onTogglePostHogPurpose={togglePostHogPurpose}
      />
    </div>
  );
}
