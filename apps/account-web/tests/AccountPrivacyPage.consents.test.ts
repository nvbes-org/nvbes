import { describe, expect, it } from 'vite-plus/test';
import { consentLabels, isRevocableConsentType } from '../src/pages/AccountPrivacyPage.consents';

describe('privacy consent presentation', () => {
  it('uses the French label for the data processing agreement', () => {
    expect(consentLabels.data_processing_agreement).toBe('Accord sur le traitement des données');
  });

  it('keeps required legal agreements non-revocable', () => {
    expect(isRevocableConsentType('terms_of_service')).toBe(false);
    expect(isRevocableConsentType('data_processing_agreement')).toBe(false);
    expect(isRevocableConsentType('privacy_policy')).toBe(false);
    expect(isRevocableConsentType('marketing_emails')).toBe(true);
  });
});
