import type { UserConsent } from '@nvbes/identity-client';
import { accountClient } from '@nvbes/identity-client';
import {
  DEFAULT_CONSENT,
  getTrackingConsent,
  revokeTrackingConsentType,
  setTrackingConsent,
} from '../tracking-consent';
import { isRevocableConsentType } from './AccountPrivacyPage.consents';

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
    if (!isRevocableConsentType(consent.consent_type)) {
      return;
    }

    const previous = consents;
    setConsents((current) => current.filter((entry) => entry.id !== consent.id));
    try {
      await accountClient.revokeConsent(consent.consent_type, consent.document_version);
      if (
        consent.consent_type === 'cookie_consent' ||
        consent.consent_type.startsWith('cookie_consent_') ||
        consent.consent_type.startsWith('analytics_')
      ) {
        setTrackingConsent(
          revokeTrackingConsentType(getTrackingConsent() ?? DEFAULT_CONSENT, consent.consent_type),
          'account-web:account-privacy:revoke',
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
