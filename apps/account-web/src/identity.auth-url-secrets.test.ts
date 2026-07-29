import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  captureAndStripSensitiveAuthUrlToken,
  clearCapturedAuthUrlToken,
  readCapturedAuthUrlToken,
} from './identity.auth-url-secrets';

afterEach(() => {
  clearCapturedAuthUrlToken('/reset-password');
  clearCapturedAuthUrlToken('/verify');
  clearCapturedAuthUrlToken('/verify-email');
  clearCapturedAuthUrlToken('/verify-result');
  vi.unstubAllGlobals();
});

describe('captureAndStripSensitiveAuthUrlToken', () => {
  it.each([
    ['/reset-password', 'reset-token'],
    ['/verify', 'verification-token'],
    ['/verify-email', 'verification-token'],
    ['/verify-result', 'verification-token'],
  ] as const)(
    'captures and removes the token for %s before observability starts',
    (route, token) => {
      const replaceState = vi.fn();
      vi.stubGlobal('window', {
        history: {
          replaceState,
          state: { navigation: 'state' },
        },
        location: {
          href: `https://account.nvbes.test${route}?token=${token}&language=fr#content`,
        },
      });

      captureAndStripSensitiveAuthUrlToken();

      expect(readCapturedAuthUrlToken(route)).toBe(token);
      expect(replaceState).toHaveBeenCalledWith(
        { navigation: 'state' },
        '',
        `${route}?language=fr#content`,
      );
    },
  );

  it('does not remove token parameters from unrelated routes', () => {
    const replaceState = vi.fn();
    vi.stubGlobal('window', {
      history: {
        replaceState,
        state: null,
      },
      location: {
        href: 'https://account.nvbes.test/oauth/callback?token=oauth-token',
      },
    });

    captureAndStripSensitiveAuthUrlToken();

    expect(replaceState).not.toHaveBeenCalled();
  });
});
