import { describe, expect, it } from 'vite-plus/test';
import { normalizeLoginReturnTo } from './identity.return-to';

describe('login return_to normalization', () => {
  const allowedOrigins = new Set(['http://localhost:3001', 'http://localhost:5175']);

  it('allows local relative paths', () => {
    expect(
      normalizeLoginReturnTo('/account/security', 'http://localhost:3001', allowedOrigins),
    ).toBe('/account/security');
  });

  it('allows configured developer portal origins', () => {
    expect(
      normalizeLoginReturnTo(
        'http://localhost:5175/portal/apps',
        'http://localhost:3001',
        allowedOrigins,
      ),
    ).toBe('http://localhost:5175/portal/apps');
  });

  it('rejects unconfigured absolute origins', () => {
    expect(
      normalizeLoginReturnTo('https://example.test/phish', 'http://localhost:3001', allowedOrigins),
    ).toBeNull();
  });
});
