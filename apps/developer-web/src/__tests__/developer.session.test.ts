import { describe, expect, it } from 'vite-plus/test';
import { buildDeveloperLoginUrl } from '../developer.session';

describe('developer session', () => {
  it('builds an Identity login URL with the current developer location as return target', () => {
    const url = new URL(buildDeveloperLoginUrl('http://localhost:5175/portal/apps?tenant=acme'));

    expect(url.origin).toBe('http://localhost:3001');
    expect(url.pathname).toBe('/login');
    expect(url.searchParams.get('return_to')).toBe('http://localhost:5175/portal/apps?tenant=acme');
  });
});
