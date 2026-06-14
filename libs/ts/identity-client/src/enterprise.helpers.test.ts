import {
  EnterpriseInvitationInputSchema,
  EnterpriseUsersResponseSchema,
} from './enterprise.schemas';

type TestApi = {
  describe: (name: string, fn: () => void) => void;
  expect: (actual: unknown) => {
    toBe: (expected: unknown) => void;
    toContain: (expected: unknown) => void;
  };
  it: (name: string, fn: () => void | Promise<void>) => void;
};

const testModuleName = 'vite-plus/test';
const { describe, expect, it } = (await import(testModuleName)) as TestApi;

describe('enterprise client schemas', () => {
  it('parses users payloads', () => {
    const parsed = EnterpriseUsersResponseSchema.parse({
      users: [],
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
});
