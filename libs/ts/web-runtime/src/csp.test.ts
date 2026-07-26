import { describe, expect, it } from 'vite-plus/test';
import { buildWebCsp } from './csp';

describe('buildWebCsp', () => {
  it('allows React style attributes without allowing production inline style elements', () => {
    const csp = buildWebCsp({
      mode: 'production',
      styleSrc: ['https://fonts.googleapis.com'],
    });

    expect(csp).toContain("style-src-elem 'self' https://fonts.googleapis.com");
    expect(csp).toContain("style-src-attr 'unsafe-inline'");
    expect(csp).not.toContain("style-src-elem 'self' 'unsafe-inline'");
    expect(csp).toContain("script-src-attr 'none'");
  });
});
