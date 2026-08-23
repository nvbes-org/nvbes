import { describe, expect, it } from 'vite-plus/test';
import { getCsp } from '../identity.vite.csp';

function directiveSources(policy: string, directiveName: string): string[] {
  const directive = policy
    .split(';')
    .map((value) => value.trim())
    .find((value) => value === directiveName || value.startsWith(`${directiveName} `));

  return directive?.split(/\s+/u).slice(1) ?? [];
}

describe('identity-web CSP', () => {
  it('allows local object-storage images in a production preview built for development', () => {
    const csp = getCsp('production', '', '', '', true);

    expect(directiveSources(csp, 'img-src')).toEqual(
      expect.arrayContaining(['http://localhost:*', 'http://127.0.0.1:*']),
    );
  });

  it('does not expose local HTTP image sources in a deployed production policy', () => {
    const csp = getCsp('production', '', '', '', false);
    const imageSources = directiveSources(csp, 'img-src');

    expect(imageSources).not.toContain('http://localhost:*');
    expect(imageSources).not.toContain('http://127.0.0.1:*');
  });

  it('allows the PostHog EU remote-config asset origin', () => {
    const csp = getCsp('production', '', 'https://eu.i.posthog.com', '', false);

    expect(directiveSources(csp, 'script-src')).toContain('https://eu-assets.i.posthog.com');
    expect(directiveSources(csp, 'connect-src')).toEqual(
      expect.arrayContaining(['https://eu.i.posthog.com', 'https://eu-assets.i.posthog.com']),
    );
  });
});
