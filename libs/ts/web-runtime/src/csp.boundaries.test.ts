import { describe, expect, it } from 'vite-plus/test';
import {
  buildWebCsp,
  cspMetaFromHeader,
  originFromUrl,
  posthogAssetsOriginFromHost,
  productionTransportHeaders,
  strictTransportSecurity,
  zodJitlessBootstrap,
} from './csp';

describe('CSP policy boundaries', () => {
  it('keeps development allowances out of production and hashes the actual bootstrap', async () => {
    const production = buildWebCsp({ mode: 'production' });
    const development = buildWebCsp({ mode: 'development', reportUri: '' });
    const hash = await crypto.subtle.digest(
      'SHA-256',
      new TextEncoder().encode(zodJitlessBootstrap),
    );
    expect(production).toContain(`'sha256-${btoa(String.fromCharCode(...new Uint8Array(hash)))}'`);
    expect(production).not.toContain('ws://localhost:*');
    expect(production).not.toContain("'unsafe-eval'");
    expect(development).toContain("script-src 'self' 'unsafe-inline' 'unsafe-eval';");
    expect(development).toContain("style-src 'self' 'unsafe-inline';");
    for (const origin of [
      'ws://localhost:*',
      'ws://127.0.0.1:*',
      'http://localhost:*',
      'http://127.0.0.1:*',
    ]) {
      expect(development).toContain(origin);
    }
    expect(development).not.toContain('sha256-');
    expect(development).not.toContain('report-uri');
  });

  it('merges configured sources without duplicate or empty values', () => {
    const sources = ["'self'", 'https://assets.test', '', 'https://assets.test'];
    const policy = buildWebCsp({
      mode: 'production',
      scriptSrc: sources,
      styleSrc: sources,
      imgSrc: sources,
      fontSrc: sources,
      connectSrc: sources,
      frameSrc: sources,
      reportUri: '/report',
    });
    const directives = new Map(policy.split('; ').map((entry) => [entry.split(' ')[0], entry]));
    for (const directive of [
      'script-src',
      'style-src',
      'style-src-elem',
      'img-src',
      'font-src',
      'connect-src',
      'frame-src',
    ]) {
      const value = directives.get(directive);
      expect(value?.match(/https:\/\/assets.test/gu)).toHaveLength(1);
      expect(value?.match(/'self'/gu)).toHaveLength(1);
    }
    expect(policy).toContain('report-uri /report;');
  });

  it.each([15, 257])('rejects a nonce of %i characters', (length) => {
    expect(() => buildWebCsp({ mode: 'production', nonce: 'a'.repeat(length) })).toThrow(
      'CSP nonce',
    );
  });

  it.each([16, 256])('accepts a trimmed nonce of %i characters', (length) => {
    const nonce = 'a'.repeat(length);
    expect(buildWebCsp({ mode: 'production', nonce: ` ${nonce} ` })).toContain(
      `'nonce-${nonce}' 'strict-dynamic'`,
    );
  });

  it('removes header-only directives for a meta tag without changing script protection', () => {
    const policy =
      "default-src 'self'; frame-ancestors 'none'; report-uri /reports; report-to csp; script-src 'self';";
    expect(cspMetaFromHeader(policy)).toBe("default-src 'self'; script-src 'self';");
    expect(cspMetaFromHeader("default-src 'none';")).toBe("default-src 'none';");
    expect(productionTransportHeaders('production')).toEqual({
      'Strict-Transport-Security': strictTransportSecurity,
    });
    expect(productionTransportHeaders('development')).toEqual({});
  });

  it.each([
    ['', ''],
    ['not a URL', ''],
    ['https://example.test:444/path?x=1', 'https://example.test:444'],
  ])('extracts an origin from %j', (input, expected) => {
    expect(originFromUrl(input)).toBe(expected);
  });

  it.each([
    ['', ''],
    ['invalid', ''],
    ['https://eu.i.posthog.com:444/path', 'https://eu-assets.i.posthog.com'],
    ['https://analytics.nvbes.test:444/path', 'https://analytics.nvbes.test:444'],
  ])('maps only the hosted analytics assets origin: %j', (input, expected) => {
    expect(posthogAssetsOriginFromHost(input)).toBe(expected);
  });
});
