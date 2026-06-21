import { useState } from 'react';
import {
  deriveConsentState,
  toggleConsentCategory,
  toggleConsentAnalyticsPurpose,
  toggleConsentVendor,
  type AnalyticsPurpose,
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

  const toggleAnalyticsPurpose = (purpose: AnalyticsPurpose) => {
    persistConsent(
      toggleConsentAnalyticsPurpose(cookieConsent, purpose),
      `identity-web:account-privacy:analytics:${purpose}`,
    );
  };

  return (
    <div className="space-y-4">
      <CookieConsentEssentialSection />
      <CookieConsentAnalyticsSection
        cookieConsent={cookieConsent}
        onToggleCategory={toggleCategory}
        onToggleVendor={toggleVendor}
        onToggleAnalyticsPurpose={toggleAnalyticsPurpose}
      />
      <CookieConsentPerformanceSection
        cookieConsent={cookieConsent}
        onToggleCategory={toggleCategory}
        onToggleVendor={toggleVendor}
        onToggleAnalyticsPurpose={toggleAnalyticsPurpose}
      />
    </div>
  );
}
