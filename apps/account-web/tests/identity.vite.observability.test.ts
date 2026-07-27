import { describe, expect, it } from 'vite-plus/test';
import { resolveFaroUrl } from '../identity.vite.observability';

describe('account-web Faro build configuration', () => {
  it('resolves the primary collector URL', () => {
    expect(resolveFaroUrl([{ VITE_FARO_URL_ACCOUNT_WEB: ' https://faro.example/collect ' }])).toBe(
      'https://faro.example/collect',
    );
  });

  it('keeps Faro optional for ordinary local development', () => {
    expect(resolveFaroUrl([{}])).toBe('');
  });

  it('rejects a release build without a collector URL', () => {
    expect(() => resolveFaroUrl([{ NVBES_REQUIRE_FARO_ACCOUNT_WEB: 'true' }])).toThrow(
      'VITE_FARO_URL_ACCOUNT_WEB is missing',
    );
  });
});
