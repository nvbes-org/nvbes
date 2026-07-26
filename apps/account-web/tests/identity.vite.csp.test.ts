import { describe, expect, it } from 'vite-plus/test';
import { getCsp } from '../identity.vite.csp';

describe('account-web CSP', () => {
  it('allows local object-storage images in a production preview built for development', () => {
    const csp = getCsp('production', '', '', '', true);

    expect(csp).toContain(
      "img-src 'self' data: blob: https: http://localhost:* http://127.0.0.1:*",
    );
  });

  it('does not expose local HTTP image sources in a deployed production policy', () => {
    const csp = getCsp('production', '', '', '', false);

    expect(csp).not.toContain('http://localhost:*');
    expect(csp).not.toContain('http://127.0.0.1:*');
  });
});
