import { EMPTY_POSTHOG_CONSENT, type PostHogPurposeConsent } from '@nvbes/web-runtime/posthog';
import { TRACKING_CONSENT_CHANGED_EVENT, type CookieConsentState } from './tracking-consent';

export function subscribeToConsent(listener: (consent: PostHogPurposeConsent) => void): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }

  const handleConsentChange = (event: Event) => {
    if (!(event instanceof CustomEvent)) {
      return;
    }

    const detail = event.detail as { consent?: CookieConsentState };
    listener(detail.consent?.posthog ?? EMPTY_POSTHOG_CONSENT);
  };

  window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
  return () => window.removeEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
}
