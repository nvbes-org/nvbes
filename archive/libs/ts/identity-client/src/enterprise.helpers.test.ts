import { describe, expect, it } from 'vite-plus/test';
import type { z } from 'zod';
import {
  activateEnterpriseBreakGlassAccount,
  revokeEnterpriseBreakGlassAccount,
  updateEnterpriseUserAccess,
} from './enterprise.client';
import {
  EnterpriseBreakGlassInputSchema,
  EnterpriseInvitationInputSchema,
  EnterpriseReactivateInputSchema,
  EnterpriseSuspendInputSchema,
  EnterpriseUsersResponseSchema,
} from './enterprise.schemas';

describe('enterprise client schemas', () => {
  it('parses users payloads', () => {
    const parsed = EnterpriseUsersResponseSchema.parse({
      users: [
        {
          ...enterpriseUserPayload(),
          break_glass: {
            procedure_reference: 'IR-2026-042',
            reason: 'Emergency tenant recovery',
            created_at: '2026-06-14T12:00:00Z',
            last_used_at: null,
          },
        },
      ],
      invitations: [],
      roles: ['owner', 'admin', 'member', 'viewer'],
      module_grants: [
        'members',
        'workspaces',
        'developers',
        'policies',
        'security',
        'billing',
        'audit',
        'drive',
      ],
      page: { cursor: null, has_more: false },
    });
    expect(parsed.roles).toContain('owner');
    expect(parsed.users[0]?.break_glass?.procedure_reference).toBe('IR-2026-042');
  });

  it('validates invitation input', () => {
    expect(
      EnterpriseInvitationInputSchema.parse({
        emails: ['admin@example.com'],
        role: 'admin',
        module_grants: ['members'],
        workspace_ids: ['workspace_123'],
      }).role,
    ).toBe('admin');
  });

  it('requires audit reasons for suspend and reactivate inputs', () => {
    expect(() => EnterpriseSuspendInputSchema.parse({})).toThrow();
    expect(() => EnterpriseReactivateInputSchema.parse({})).toThrow();
    expect(() => EnterpriseSuspendInputSchema.parse({ reason: '   ' })).toThrow();
    expect(() => EnterpriseReactivateInputSchema.parse({ reason: '   ' })).toThrow();

    expect(EnterpriseSuspendInputSchema.parse({ reason: 'security_review' }).reason).toBe(
      'security_review',
    );
    expect(EnterpriseReactivateInputSchema.parse({ reason: 'review_complete' }).reason).toBe(
      'review_complete',
    );
  });

  it('requires break-glass reason and procedure input', () => {
    expect(() => EnterpriseBreakGlassInputSchema.parse({ reason: 'incident' })).toThrow();
    expect(() =>
      EnterpriseBreakGlassInputSchema.parse({
        procedure_reference: 'IR-2026-042',
        reason: '   ',
      }),
    ).toThrow();
    expect(
      EnterpriseBreakGlassInputSchema.parse({
        procedure_reference: 'IR-2026-042',
        reason: 'incident',
      }).procedure_reference,
    ).toBe('IR-2026-042');
  });

  it('patches enterprise user access through the versioned API path', async () => {
    const requests: RecordedRequest[] = [];
    const http = {
      request<T>(path: string, schema: z.ZodType<T>, options: TestRequestOptions = {}): Promise<T> {
        requests.push({
          body: JSON.stringify(options.body),
          method: options.method,
          path,
        });

        return Promise.resolve(schema.parse({ user: enterpriseUserPayload() }));
      },
    } as unknown as Parameters<typeof updateEnterpriseUserAccess>[0];

    await updateEnterpriseUserAccess(http, 'user/123', {
      role: 'admin',
      module_grants: ['members'],
      workspace_ids: ['workspace_123'],
    });

    expect(requests[0]?.path).toBe('/api/v1/enterprise/users/user%2F123/access');
    expect(requests[0]?.method).toBe('PATCH');
    expect(requests[0]?.body).toBe(
      JSON.stringify({
        role: 'admin',
        module_grants: ['members'],
        workspace_ids: ['workspace_123'],
      }),
    );
  });

  it('posts enterprise break-glass account changes through encoded user paths', async () => {
    const requests: RecordedRequest[] = [];
    const http = {
      post<T>(path: string, schema: z.ZodType<T>, body: unknown): Promise<T> {
        requests.push({
          body: JSON.stringify(body),
          method: 'POST',
          path,
        });

        return Promise.resolve(schema.parse({ user: enterpriseUserPayload() }));
      },
    } as unknown as Parameters<typeof activateEnterpriseBreakGlassAccount>[0];

    await activateEnterpriseBreakGlassAccount(http, 'user/123', {
      procedure_reference: 'IR-2026-042',
      reason: 'incident',
    });
    await revokeEnterpriseBreakGlassAccount(http, 'user/123', {
      reason: 'rotation complete',
    });

    expect(requests[0]?.path).toBe('/api/v1/enterprise/users/user%2F123/break-glass');
    expect(requests[1]?.path).toBe('/api/v1/enterprise/users/user%2F123/break-glass/revoke');
  });
});

type RecordedRequest = {
  body: string | undefined;
  method: string | undefined;
  path: string;
};

type TestRequestOptions = {
  body?: unknown;
  method?: string;
};

function enterpriseUserPayload() {
  return {
    id: 'user_123',
    email: 'admin@example.com',
    display_name: 'Admin User',
    role: 'admin',
    module_grants: ['members'],
    workspace_ids: ['workspace_123'],
    status: 'active',
    mfa_enabled: true,
    created_at: '2026-06-14T12:00:00Z',
  };
}
