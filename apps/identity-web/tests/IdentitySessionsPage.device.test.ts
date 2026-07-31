import type { AccountSession } from '@nvbes/identity-client';
import { describe, expect, it } from 'vite-plus/test';

import {
  formatActiveSessionCount,
  groupSessionsByDevice,
  parseSessionClient,
} from '../src/pages/IdentitySessionsPage.device';

function session(overrides: Partial<AccountSession>): AccountSession {
  return {
    id: 'session-1',
    tenant_id: null,
    organization_id: null,
    workspace_id: null,
    workspace_region: null,
    created_at: '2026-07-27T10:00:00Z',
    last_seen_at: '2026-07-27T10:30:00Z',
    expires_at: '2026-08-27T10:00:00Z',
    revoked_at: null,
    ip: '203.0.113.10',
    geo_country_code: 'FR',
    user_agent: 'Chrome/126',
    client: {
      browser: 'Chrome',
      browser_version: 126,
      os: 'Windows',
      os_version: '10',
      device: null,
      device_type: 'desktop',
    },
    device_id: null,
    device_trust_level: 'recognized',
    device_trust_score: 75,
    risk_score: 10,
    risk_decision: 'allow',
    risk_confirmed_at: null,
    current: false,
    ...overrides,
  };
}

describe('session client presentation', () => {
  it('uses correct French singular and plural session labels', () => {
    expect(formatActiveSessionCount(1)).toBe('1 session active');
    expect(formatActiveSessionCount(2)).toBe('2 sessions actives');
  });

  it('uses normalized backend metadata', () => {
    expect(parseSessionClient(session({}).client)).toEqual({
      browser: 'Chrome',
      browserVersion: 126,
      browserKey: 'chrome',
      os: 'Windows',
      osVersion: '10',
      osKey: 'windows',
      device: null,
      deviceType: 'desktop',
    });
  });

  it('groups browser upgrades as the same device family', () => {
    const grouped = groupSessionsByDevice([
      session({ id: 'old', user_agent: 'Chrome/126' }),
      session({
        id: 'new',
        user_agent: 'Chrome/127',
        client: {
          ...session({}).client!,
          browser_version: 127,
        },
      }),
    ]);

    expect(grouped.recognizedDevices).toHaveLength(1);
    expect(grouped.recognizedDevices[0]?.sessions).toHaveLength(2);
  });
});
