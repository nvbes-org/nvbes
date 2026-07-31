import { describe, expect, it } from 'vite-plus/test';
import { emailHasSupportedFormat } from './identity.email.policy';

describe('email policy', () => {
  it('accepts a supported email address', () => {
    expect(emailHasSupportedFormat(' utilisateur@nvbes.fr ')).toBe(true);
  });

  it('rejects incomplete email addresses', () => {
    expect(emailHasSupportedFormat('utilisateur')).toBe(false);
    expect(emailHasSupportedFormat('@nvbes.fr')).toBe(false);
    expect(emailHasSupportedFormat('utilisateur@')).toBe(false);
  });
});
