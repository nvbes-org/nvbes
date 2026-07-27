import { describe, expect, it } from 'vite-plus/test';
import { buildWebCsp, posthogAssetsOriginFromHost } from './csp';

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

describe('posthogAssetsOriginFromHost', () => {
  it('maps the PostHog EU ingestion host to its remote-config asset origin', () => {
    expect(posthogAssetsOriginFromHost('https://eu.i.posthog.com')).toBe(
      'https://eu-assets.i.posthog.com',
    );
  });

  it('keeps a self-hosted PostHog origin on the same host', () => {
    expect(posthogAssetsOriginFromHost('https://analytics.example.com')).toBe(
      'https://analytics.example.com',
    );
  });

  it('returns no source for an invalid or absent host', () => {
    expect(posthogAssetsOriginFromHost('')).toBe('');
    expect(posthogAssetsOriginFromHost('not a URL')).toBe('');
  });
});
