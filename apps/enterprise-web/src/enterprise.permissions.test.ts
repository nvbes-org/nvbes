import {
  canManagePolicies,
  canManageUsers,
  describeModuleGrant,
  isLastOwnerRemoval,
} from './enterprise.permissions';

type TestExpectation = {
  toBe(expected: unknown): void;
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

describe('enterprise permissions', () => {
  it('allows owners and members-admins to manage users', () => {
    expect(canManageUsers({ role: 'owner', module_grants: [] })).toBe(true);
    expect(canManageUsers({ role: 'admin', module_grants: ['members'] })).toBe(true);
  });

  it('blocks users without members administration access', () => {
    expect(canManageUsers({ role: 'admin', module_grants: ['billing'] })).toBe(false);
    expect(canManageUsers({ role: 'member', module_grants: ['members'] })).toBe(false);
  });

  it('allows owners and policies-admins to simulate policies', () => {
    expect(canManagePolicies({ role: 'owner', module_grants: [] })).toBe(true);
    expect(canManagePolicies({ role: 'admin', module_grants: ['policies'] })).toBe(true);
  });

  it('blocks users without policies administration access', () => {
    expect(canManagePolicies({ role: 'admin', module_grants: ['members'] })).toBe(false);
    expect(canManagePolicies({ role: 'member', module_grants: ['policies'] })).toBe(false);
  });

  it('blocks last owner removal', () => {
    expect(
      isLastOwnerRemoval({ currentOwnerCount: 1, selectedRole: 'owner', nextRole: 'admin' }),
    ).toBe(true);
  });

  it('allows owner changes that keep at least one owner', () => {
    expect(
      isLastOwnerRemoval({ currentOwnerCount: 2, selectedRole: 'owner', nextRole: 'admin' }),
    ).toBe(false);
    expect(
      isLastOwnerRemoval({ currentOwnerCount: 1, selectedRole: 'owner', nextRole: 'owner' }),
    ).toBe(false);
    expect(
      isLastOwnerRemoval({ currentOwnerCount: 1, selectedRole: 'admin', nextRole: 'member' }),
    ).toBe(false);
  });

  it('labels grants', () => {
    expect(describeModuleGrant('members')).toBe('Members');
    expect(describeModuleGrant('audit')).toBe('Audit logs');
  });
});
