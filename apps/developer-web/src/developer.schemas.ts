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

export const DeveloperHealthStatusSchema = z.enum(['passing', 'warning', 'failing', 'unknown']);
export const DeveloperMarketplaceStatusSchema = z
  .enum(['pending', 'approved', 'rejected', 'suspended'])
  .nullable();

export const DeveloperOAuthClientSchema = z.object({
  client_id: z.string(),
  name: z.string(),
  status: z.string(),
  marketplace_status: DeveloperMarketplaceStatusSchema,
  consent_screen_configured: z.boolean(),
  redirect_uri_count: z.number().int().nonnegative(),
  allowed_scopes: z.array(z.string()),
  health_status: DeveloperHealthStatusSchema,
});

export const DeveloperOAuthClientsSchema = z.object({
  oauth_clients: z.array(DeveloperOAuthClientSchema),
});

export const DeveloperMarketplaceAppSchema = z.object({
  client_id: z.string(),
  name: z.string(),
  status: z.enum(['pending', 'approved', 'rejected', 'suspended']),
  review_reason: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const DeveloperMarketplaceAppsSchema = z.object({
  apps: z.array(DeveloperMarketplaceAppSchema),
});

export const DeveloperScopeRegistryEntrySchema = z.object({
  scope_key: z.string(),
  display_name: z.string(),
  description: z.string(),
  risk: z.enum(['low', 'medium', 'high', 'restricted']),
  owner_team: z.string(),
  lifecycle: z.enum(['proposed', 'active', 'deprecated', 'retired']),
  allowed_audiences: z.array(z.string()),
});

export const DeveloperScopeRegistrySchema = z.object({
  scopes: z.array(DeveloperScopeRegistryEntrySchema),
});

export type DeveloperRole = z.infer<typeof DeveloperRoleSchema>;
export type DeveloperPermission = z.infer<typeof DeveloperPermissionSchema>;
export type DeveloperContext = z.infer<typeof DeveloperContextSchema>;
export type DeveloperOverview = z.infer<typeof DeveloperOverviewSchema>;
export type DeveloperOAuthClient = z.infer<typeof DeveloperOAuthClientSchema>;
export type DeveloperMarketplaceApp = z.infer<typeof DeveloperMarketplaceAppSchema>;
export type DeveloperScopeRegistryEntry = z.infer<typeof DeveloperScopeRegistryEntrySchema>;
