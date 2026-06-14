import { describe, expect, it } from 'vite-plus/test';

import { HostedLoginDecisionSchema, readHostedStateId } from './identity.universal-login.api';

describe('Universal Login API helpers', () => {
  it('parses consent required decisions', () => {
    const parsed = HostedLoginDecisionSchema.parse({
      kind: 'consent_required',
      state_id: 'hosted_123',
      client: { client_id: 'drive_web', name: 'nvbes Drive' },
      scope: 'openid profile email',
    });

    expect(parsed.kind).toBe('consent_required');
    expect(parsed.client.name).toBe('nvbes Drive');
  });

  it('reads hosted state id from login URL parameters', () => {
    const searchParams = new URLSearchParams('?state_id=hosted_123');

    expect(readHostedStateId(searchParams)).toBe('hosted_123');
  });
});
