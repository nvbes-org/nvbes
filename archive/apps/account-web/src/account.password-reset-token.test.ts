import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  captureAndStripPasswordResetToken,
  clearPasswordResetToken,
  readPasswordResetToken,
} from './account.password-reset-token';

afterEach(() => {
  clearPasswordResetToken();
  vi.unstubAllGlobals();
});

describe('password reset URL token', () => {
  it('captures the token before observability and removes it from the URL', () => {
    const replaceState = vi.fn();
    vi.stubGlobal('window', {
      history: { replaceState, state: null },
      location: {
        href: 'https://account.example/reset-password?token=secret&language=fr#form',
        pathname: '/reset-password',
      },
    });

    captureAndStripPasswordResetToken();

    expect(readPasswordResetToken()).toBe('secret');
    expect(replaceState).toHaveBeenCalledWith(null, '', '/reset-password?language=fr#form');
  });
});
