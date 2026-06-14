import { buildEnterpriseInvitationInput, parseInvitationEmails } from './enterprise.invites';

type TestExpectation = {
  toEqual(expected: unknown): void;
  toThrow(expected?: string): void;
};

type TestApi = {
  describe(name: string, fn: () => void): void;
  expect(actual: unknown): TestExpectation;
  it(name: string, fn: () => void): void;
};

const testModuleName = 'vite-plus/test';
const testApi = (await import(testModuleName)) as TestApi;
const describe: TestApi['describe'] = (name, fn) => testApi.describe(name, fn);
const expect: TestApi['expect'] = (actual) => testApi.expect(actual);
const it: TestApi['it'] = (name, fn) => testApi.it(name, fn);

describe('enterprise invitations', () => {
  it('splits comma and newline separated recipients', () => {
    expect(
      parseInvitationEmails('owner@example.com, admin@example.com\nmember@example.com'),
    ).toEqual(['owner@example.com', 'admin@example.com', 'member@example.com']);
  });

  it('trims recipients and removes empty values', () => {
    expect(parseInvitationEmails('  owner@example.com, \n\n admin@example.com  ')).toEqual([
      'owner@example.com',
      'admin@example.com',
    ]);
  });

  it('rejects empty recipient lists', () => {
    expect(() => parseInvitationEmails(' , \n ')).toThrow(
      'At least one invitation recipient is required.',
    );
  });

  it('builds enterprise invitation input with contract validation', () => {
    expect(
      buildEnterpriseInvitationInput({
        emailInput: 'owner@example.com,\nadmin@example.com',
        role: 'admin',
        module_grants: ['members', 'billing'],
        workspace_ids: ['workspace_123'],
      }),
    ).toEqual({
      emails: ['owner@example.com', 'admin@example.com'],
      role: 'admin',
      module_grants: ['members', 'billing'],
      workspace_ids: ['workspace_123'],
    });
  });

  it('rejects invalid recipient email addresses', () => {
    expect(() =>
      buildEnterpriseInvitationInput({
        emailInput: 'not-an-email',
        role: 'member',
        module_grants: ['drive'],
        workspace_ids: [],
      }),
    ).toThrow();
  });
});
