import type { UserConsent } from '@nvbes/identity-client';
import { identityClient } from '@nvbes/identity-client';
import { DECLINE_ALL_CONSENT, setTrackingConsent } from '../tracking-consent';

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
        setTrackingConsent(DECLINE_ALL_CONSENT, 'identity-web:account-privacy:revoke');
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
