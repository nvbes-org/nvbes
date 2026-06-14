import { z } from 'zod';

export const DeveloperRoleSchema = z.enum([
  'developer_admin',
  'app_manager',
  'webhook_manager',
  'log_viewer',
  'integration_tester',
  'docs_viewer',
]);

export const DeveloperPermissionSchema = z.enum([
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
]);

export const DeveloperContextSchema = z.object({
  tenantId: z.string().uuid(),
  principalId: z.string().uuid(),
  displayName: z.string(),
  email: z.string().email(),
  roles: z.array(DeveloperRoleSchema),
  permissions: z.array(DeveloperPermissionSchema),
});

export const DeveloperOverviewSchema = z.object({
  tenantId: z.string().uuid(),
  oauthClients: z.number().int().nonnegative(),
  marketplacePending: z.number().int().nonnegative(),
  highRiskScopes: z.number().int().nonnegative(),
  failedWebhookDeliveries: z.number().int().nonnegative(),
  unhealthyIntegrations: z.number().int().nonnegative(),
  activeSandboxes: z.number().int().nonnegative(),
});

export type DeveloperRole = z.infer<typeof DeveloperRoleSchema>;
export type DeveloperPermission = z.infer<typeof DeveloperPermissionSchema>;
export type DeveloperContext = z.infer<typeof DeveloperContextSchema>;
export type DeveloperOverview = z.infer<typeof DeveloperOverviewSchema>;
