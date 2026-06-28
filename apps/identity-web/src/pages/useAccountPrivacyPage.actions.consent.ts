import type { UserConsent } from '@nvbes/identity-client';
import { identityClient } from '@nvbes/identity-client';
import {
  DEFAULT_CONSENT,
  getTrackingConsent,
  setTrackingConsent,
  type CookieConsentState,
} from '../tracking-consent';

function consentAfterCookieRevocation(consentType: string): CookieConsentState {
  const current = getTrackingConsent() ?? DEFAULT_CONSENT;
  const next: CookieConsentState = {
    categories: { ...current.categories },
    vendors: { ...current.vendors },
    analytics: { ...current.analytics },
  };

  if (consentType === 'cookie_consent') {
    next.categories.analytics = false;
    next.categories.performance = false;
    next.vendors.posthog = false;
    next.vendors.sentry = false;
    next.vendors.grafana = false;
  }

  if (
    consentType === 'cookie_consent_analytics' ||
    consentType === 'cookie_consent_vendor_analytics'
  ) {
    next.categories.analytics = false;
    next.vendors.posthog = false;
  }

  if (
    consentType === 'cookie_consent_performance' ||
    consentType === 'cookie_consent_vendor_error_reporting'
  ) {
    next.categories.performance = false;
    next.vendors.sentry = false;
    next.vendors.grafana = false;
  }

  if (consentType === 'cookie_consent_vendor_posthog') {
    next.vendors.posthog = false;
  }

  if (consentType === 'cookie_consent_vendor_sentry') {
    next.vendors.sentry = false;
  }

  if (consentType === 'cookie_consent_vendor_grafana') {
    next.vendors.grafana = false;
  }

  next.analytics = {
    productAnalytics: next.categories.analytics || next.vendors.posthog,
    autocaptureHeatmaps: next.categories.analytics || next.vendors.posthog,
    sessionReplay: next.categories.analytics || next.vendors.posthog,
    surveysFeedback: next.categories.analytics || next.vendors.posthog,
    featureFlags: next.categories.analytics || next.vendors.posthog,
    errorTracking: next.categories.performance || next.vendors.sentry || next.vendors.grafana,
  };

  return next;
}

export function buildAccountPrivacyConsentActions({
  consents,
  setConsents,
  setCookieConsentRevision,
}: {
  consents: UserConsent[];
  setConsents: React.Dispatch<React.SetStateAction<UserConsent[]>>;
  setCookieConsentRevision: React.Dispatch<React.SetStateAction<number>>;
}) {
  const handleRevoke = async (consent: UserConsent) => {
    const previous = consents;
    setConsents((current) => current.filter((entry) => entry.id !== consent.id));
    try {
      await identityClient.revokeConsent(consent.consent_type, consent.document_version);
      if (
        consent.consent_type === 'cookie_consent' ||
        consent.consent_type.startsWith('cookie_consent_')
      ) {
        setTrackingConsent(
          consentAfterCookieRevocation(consent.consent_type),
          'identity-web:account-privacy:revoke',
        );
        setCookieConsentRevision((revision) => revision + 1);
      }
    } catch {
      setConsents(previous);
    }
  };

  return {
    handleRevoke,
  };
}
