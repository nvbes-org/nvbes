import { describe, expect, it } from 'vite-plus/test';
import { DebugDeveloperTokenSchema, DeveloperHealthChecksSchema } from '../developer.schemas';
import { countUnhealthyChecks } from '../pages/HealthChecksPage.helpers';
import {
  describeAccessDecision,
  summarizeAccessDecision,
  summarizeExpiration,
} from '../pages/TokenDebuggerPage.helpers';

describe('developer token debugger and health checks', () => {
  it('summarizes token access decisions', () => {
    expect(summarizeAccessDecision(null)).toBe('No token inspected');
    expect(describeAccessDecision(null)).toContain('Paste an access token');
    expect(
      summarizeAccessDecision({
        active: false,
        access_decision: 'tenant_mismatch',
        claims: null,
        token_hash_prefix: 'abc123',
      }),
    ).toBe('Tenant mismatch');
  });

  it('summarizes expiration relative to a fixed clock', () => {
    const now = new Date('2026-06-14T09:00:00Z');

    expect(summarizeExpiration('2026-06-14T10:00:00Z', now)).toContain('Expires in 1 hour');
    expect(summarizeExpiration('2026-06-14T08:58:00Z', now)).toContain('Expired 2 minutes ago');
  });

  it('parses token debug responses without raw token material', () => {
    const parsed = DebugDeveloperTokenSchema.parse({
      active: true,
      access_decision: 'allowed',
      token_hash_prefix: '9af23d95f704',
      claims: {
        subject: 'principal-1',
        tenant_id: '018f3c58-7dd0-7000-8000-000000000001',
        workspace_id: null,
        client_id: 'client-1',
        scopes: ['openid', 'profile'],
        audience: 'nvbes-identity-api',
        issuer: 'nvbes-identity',
        expires_at: '2026-06-14T10:00:00Z',
        issued_at: '2026-06-14T09:00:00Z',
        not_before: '2026-06-14T09:00:00Z',
        token_type: 'access',
        amr: ['pwd'],
        acr: null,
      },
    });

    expect(parsed.token_hash_prefix).toBe('9af23d95f704');
    expect(parsed.claims?.scopes).toEqual(['openid', 'profile']);
  });

  it('counts unhealthy integration checks', () => {
    const parsed = DeveloperHealthChecksSchema.parse({
      checks: [
        {
          id: '018f3c58-7dd0-7000-8000-000000000001',
          target_type: 'oauth_client',
          target_id: 'client-1',
          check_kind: 'redirects',
          status: 'passing',
          summary: 'ok',
          checked_at: '2026-06-14T09:00:00Z',
        },
        {
          id: '018f3c58-7dd0-7000-8000-000000000002',
          target_type: 'webhook',
          target_id: 'endpoint-1',
          check_kind: 'webhook',
          status: 'failing',
          summary: 'bad',
          checked_at: '2026-06-14T09:00:00Z',
        },
      ],
    });

    expect(countUnhealthyChecks(parsed.checks)).toBe(1);
  });
});
