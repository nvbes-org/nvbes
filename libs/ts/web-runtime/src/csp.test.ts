import { describe, expect, it } from 'vite-plus/test';
import { buildWebCsp } from './csp';

describe('buildWebCsp', () => {
  it('allows the runtime style elements required by the SPA component stack', () => {
    const csp = buildWebCsp({
      mode: 'production',
      styleSrc: ['https://fonts.googleapis.com'],
    });

    expect(csp).toContain("style-src-elem 'self' 'unsafe-inline' https://fonts.googleapis.com");
    expect(csp).toContain("style-src-attr 'unsafe-inline'");
    expect(csp).toContain("script-src-attr 'none'");
  });

  it('does not upgrade local HTTP object-storage URLs to HTTPS', () => {
    const csp = buildWebCsp({ mode: 'production' });

    expect(csp).not.toContain('upgrade-insecure-requests');
  });
});
