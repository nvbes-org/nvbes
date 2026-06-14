import { describe, expect, it } from 'vite-plus/test';

import { shouldShowHostedConsent } from './useUniversalLogin';

describe('Universal Login flow helpers', () => {
  it('shows hosted consent when backend requires it', () => {
    expect(
      shouldShowHostedConsent({
        kind: 'consent_required',
        state_id: 'hosted_123',
        client: { client_id: 'drive_web', name: 'nvbes Drive' },
        scope: 'openid profile email',
      }),
    ).toBe(true);
  });
});
