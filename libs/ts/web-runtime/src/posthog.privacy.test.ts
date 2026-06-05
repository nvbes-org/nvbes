import { describe, expect, it } from 'vite-plus/test';
import {
  DEFAULT_BLOCKED_ROUTE_PATTERNS,
  hasAnyPostHogConsent,
  normalizePostHogError,
  sanitizePostHogProperties,
} from './posthog.privacy';
import { EMPTY_POSTHOG_CONSENT } from './posthog.types';

describe('PostHog privacy helpers', () => {
  it('detects purpose-level consent', () => {
    expect(hasAnyPostHogConsent(EMPTY_POSTHOG_CONSENT)).toBe(false);
    expect(
      hasAnyPostHogConsent({
        ...EMPTY_POSTHOG_CONSENT,
        featureFlags: true,
      }),
    ).toBe(true);
  });

  it('keeps only allowlisted safe properties and pseudonymizes IDs', async () => {
    const sanitized = await sanitizePostHogProperties(
      {
        email: 'user@example.com',
        file_name: 'contract.pdf',
        plan_code: 'team',
        route_path: '/dashboard',
        token: 'secret',
        user_id: '018f2f61-4875-7f7a-8bc8-8f70a73d2b1f',
        workspace_id: '018f2f62-4875-7f7a-8bc8-8f70a73d2b1f',
      },
      async (prefix, id) => `${prefix}_${id.slice(0, 8)}`,
    );

    expect(sanitized).toEqual({
      plan_code: 'team',
      route_path: '/dashboard',
      user_id: 'usr_018f2f61',
      workspace_id: 'wks_018f2f62',
    });
  });

  it('drops sensitive property values', async () => {
    const sanitized = await sanitizePostHogProperties(
      {
        error_message: 'failed for user@example.com',
        feature_flag: 'new-nav',
        variant: 'control',
      },
      async (prefix, id) => `${prefix}_${id}`,
    );

    expect(sanitized).toEqual({
      feature_flag: 'new-nav',
      variant: 'control',
    });
  });

  it('blocks replay/error capture on sensitive routes', () => {
    const isBlocked = (path: string) =>
      DEFAULT_BLOCKED_ROUTE_PATTERNS.some((pattern) => pattern.test(path));

    expect(isBlocked('/login')).toBe(true);
    expect(isBlocked('/account/privacy')).toBe(true);
    expect(isBlocked('/dashboard')).toBe(false);
  });

  it('scrubs sensitive browser error messages', () => {
    expect(normalizePostHogError(new Error('token abc.def.ghi')).message).toBe(
      'Redacted error message',
    );
    expect(normalizePostHogError(new Error('plain failure')).message).toBe('plain failure');
  });
});
