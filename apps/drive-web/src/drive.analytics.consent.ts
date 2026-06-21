import {
  EMPTY_ANALYTICS_CONSENT,
  type AnalyticsPurposeConsent,
} from '@nvbes/web-runtime/analytics';
import { TRACKING_CONSENT_CHANGED_EVENT, type CookieConsentState } from './tracking-consent';

export function subscribeToConsent(
  listener: (consent: AnalyticsPurposeConsent) => void,
): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }

  const handleConsentChange = (event: Event) => {
    if (!(event instanceof CustomEvent)) {
      return;
    }

    const detail = event.detail as { consent?: CookieConsentState };
    listener(detail.consent?.analytics ?? EMPTY_ANALYTICS_CONSENT);
  };

  window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
  return () => window.removeEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
}
