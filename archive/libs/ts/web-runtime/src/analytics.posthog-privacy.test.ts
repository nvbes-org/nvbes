import { describe, expect, it } from 'vite-plus/test';
import { stripPostHogUrlSecrets } from './analytics.posthog-privacy';

describe('stripPostHogUrlSecrets', () => {
  it('removes query strings, fragments, and credentials from automatic URL properties', () => {
    const event = stripPostHogUrlSecrets({
      event: '$pageview',
      properties: {
        $current_url:
          'https://user:password@account.nvbes.test/reset-password?token=secret-token#reset-form',
        $session_entry_url: 'https://account.nvbes.test/verify-result?token=verification-token',
        source: 'account-web',
      },
      uuid: '123e4567-e89b-12d3-a456-426614174000',
    });

    expect(event?.properties).toEqual({
      $current_url: 'https://account.nvbes.test/reset-password',
      $session_entry_url: 'https://account.nvbes.test/verify-result',
      source: 'account-web',
    });
  });

  it('leaves events without a current URL unchanged', () => {
    const event = {
      event: 'account.updated',
      properties: { source: 'account-web' },
      uuid: '123e4567-e89b-12d3-a456-426614174001',
    };

    expect(stripPostHogUrlSecrets(event)).toBe(event);
    expect(stripPostHogUrlSecrets(null)).toBeNull();
  });
});
