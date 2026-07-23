import { deriveConsentState, toggleConsentAnalyticsPurpose } from '@nvbes/web-runtime';
import { useState } from 'react';
import {
  type CookieConsentState,
  DEFAULT_CONSENT,
  getTrackingConsent,
  setTrackingConsent,
} from '../tracking-consent';
import {
  CookieConsentAnalyticsSection,
  CookieConsentEssentialSection,
  CookieConsentPerformanceSection,
} from './CookieConsentSettings.sections';

export function CookieConsentSettings({
  onConsentChange,
}: {
  onConsentChange?: (consent: CookieConsentState) => void;
}) {
  const [cookieConsent, setCookieConsent] = useState<CookieConsentState>(
    () => getTrackingConsent() || DEFAULT_CONSENT,
  );

  const persistConsent = (next: CookieConsentState, source: string) => {
    const derived = deriveConsentState(next);
    setCookieConsent(derived);
    setTrackingConsent(derived, source);
    onConsentChange?.(derived);
  };

  const togglePurpose = (purpose: keyof CookieConsentState['analytics']) => {
    persistConsent(
      toggleConsentAnalyticsPurpose(cookieConsent, purpose),
      `account-web:account-privacy:${purpose}`,
    );
  };

  return (
    <div className="flex flex-col gap-6">
      <CookieConsentEssentialSection />
      <CookieConsentAnalyticsSection
        cookieConsent={cookieConsent}
        onTogglePurpose={togglePurpose}
      />
      <CookieConsentPerformanceSection
        cookieConsent={cookieConsent}
        onTogglePurpose={togglePurpose}
      />
    </div>
  );
}
