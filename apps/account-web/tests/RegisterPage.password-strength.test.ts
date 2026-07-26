import { describe, expect, it } from 'vite-plus/test';

import { estimatePasswordStrength } from '../src/pages/RegisterPage.password-strength';

describe('estimatePasswordStrength', () => {
  it('uses the configured common and French dictionaries', () => {
    expect(estimatePasswordStrength('motdepasse').score).toBeLessThan(3);
  });

  it('accepts a sufficiently strong passphrase', () => {
    expect(estimatePasswordStrength('Orbite-Cuivre-Lagon-7391!').score).toBeGreaterThanOrEqual(3);
  });
});
