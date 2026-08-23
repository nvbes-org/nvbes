import { describe, expect, it } from 'vite-plus/test';
import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  passwordCharacterCount,
  passwordHasSupportedLength,
} from './identity.password.policy';

describe('password policy', () => {
  it('accepts NIST-length passphrases without composition requirements', () => {
    expect(passwordHasSupportedLength('rivière nuage cuivre galaxie')).toBe(true);
  });

  it('counts Unicode code points consistently with the backend', () => {
    expect(passwordCharacterCount('🔐'.repeat(MIN_PASSWORD_LENGTH))).toBe(MIN_PASSWORD_LENGTH);
    expect(passwordHasSupportedLength('🔐'.repeat(MIN_PASSWORD_LENGTH))).toBe(true);
  });

  it('enforces the supported length envelope', () => {
    expect(passwordHasSupportedLength('a'.repeat(MIN_PASSWORD_LENGTH - 1))).toBe(false);
    expect(passwordHasSupportedLength('a'.repeat(MAX_PASSWORD_LENGTH + 1))).toBe(false);
  });
});
