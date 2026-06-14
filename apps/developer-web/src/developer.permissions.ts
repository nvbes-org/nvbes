import type { DeveloperContext, DeveloperPermission } from './developer.schemas';

export const DEVELOPER_PERMISSIONS = [
  'docs.read',
  'apps.read',
  'apps.create',
  'apps.update',
  'apps.revoke',
  'marketplace.read',
  'marketplace.submit',
  'marketplace.review',
  'scopes.read',
  'scopes.manage',
  'secrets.rotate',
  'service_accounts.read',
  'service_accounts.manage',
  'webhooks.read',
  'webhooks.manage',
  'webhooks.replay',
  'logs.read',
  'tokens.inspect',
  'health_checks.read',
  'health_checks.run',
  'sandbox.use',
  'rbac.manage',
] as const satisfies readonly DeveloperPermission[];

type PermissionContext = Pick<DeveloperContext, 'roles' | 'permissions'>;

export function canUseDeveloperPermission(
  context: PermissionContext,
  permission: DeveloperPermission,
): boolean {
  if (context.roles.includes('developer_admin')) {
    return DEVELOPER_PERMISSIONS.includes(permission);
  }

  return context.permissions.includes(permission);
}
