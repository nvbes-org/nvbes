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
  'developer.apps.read',
  'developer.apps.create',
  'developer.apps.update_redirects',
  'developer.apps.revoke',
  'developer.webhooks.read',
  'developer.webhooks.manage',
  'developer.logs.read',
  'developer.tokens.inspect',
  'developer.oauth.playground',
  'developer.rbac.manage',
  'developer.docs.read',
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

export const DeveloperMeSchema = z.object({
  tenant_id: z.string(),
  roles: z.array(DeveloperRoleSchema),
  permissions: z.array(DeveloperPermissionSchema),
});

export const DeveloperAppSchema = z.object({
  id: z.string(),
  client_id: z.string(),
  name: z.string(),
  redirect_uris: z.array(z.string()),
  client_type: z.string(),
  created_at: z.string(),
  last_used_at: z.string().nullable().optional(),
});

export const DeveloperAppsResponseSchema = z.object({
  apps: z.array(DeveloperAppSchema),
});

export const CreateDeveloperAppResponseSchema = z.object({
  app: DeveloperAppSchema,
  client_secret: z.string(),
});

export const PortalDeveloperWebhookEndpointSchema = z.object({
  id: z.string(),
  name: z.string(),
  url: z.string(),
  status: z.string(),
  events: z.array(z.enum(['user.created', 'login.failed', 'session.revoked', 'client.created'])),
  signing_secret_last4: z.string(),
  created_at: z.string(),
});

export const DeveloperWebhooksResponseSchema = z.object({
  endpoints: z.array(PortalDeveloperWebhookEndpointSchema),
});

export const CreateDeveloperWebhookEndpointResponseSchema = z.object({
  endpoint: PortalDeveloperWebhookEndpointSchema,
  signing_secret: z.string(),
});

export const InspectDeveloperTokenResponseSchema = z.object({
  active: z.boolean(),
  subject: z.string().nullable(),
  client_id: z.string().nullable(),
  tenant_id: z.string().nullable(),
  scopes: z.array(z.string()),
  expires_at: z.string().nullable(),
});

export const OAuthPlaygroundExchangeResponseSchema = z.object({
  token_type: z.string(),
  expires_in: z.number(),
  scope: z.string(),
});

export const PortalDeveloperLogEntrySchema = z.object({
  id: z.string(),
  event_type: z.string(),
  user_id: z.string().nullable(),
  client_id: z.string().nullable(),
  tenant_id: z.string().nullable(),
  created_at: z.string(),
});

export const DeveloperPortalLogsResponseSchema = z.object({
  logs: z.array(PortalDeveloperLogEntrySchema),
});

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

export const DeveloperConsentScreenSchema = z.object({
  client_id: z.string(),
  product_name: z.string(),
  logo_url: z.string().nullable(),
  support_url: z.string().nullable(),
  privacy_url: z.string().nullable(),
  terms_url: z.string().nullable(),
  description: z.string(),
  configured: z.boolean(),
  updated_at: z.string().nullable(),
});

export const DeveloperServiceAccountSchema = z.object({
  principal_id: z.string().uuid(),
  name: z.string(),
  description: z.string().nullable(),
  role: z.string(),
  status: z.string(),
  workspace_id: z.string().uuid().nullable(),
  oauth_client_count: z.number().int().nonnegative(),
  last_rotated_at: z.string().nullable(),
});

export const DeveloperServiceAccountsSchema = z.object({
  service_accounts: z.array(DeveloperServiceAccountSchema),
});

export const DeveloperSecretVersionSchema = z.object({
  id: z.string().uuid(),
  client_id: z.string(),
  status: z.enum(['active', 'overlap', 'expired', 'revoked']),
  secret_last4: z.string(),
  created_at: z.string(),
  expires_at: z.string().nullable(),
  revoked_at: z.string().nullable(),
});

export const DeveloperSecretVersionsSchema = z.object({
  secret_versions: z.array(DeveloperSecretVersionSchema),
});

export const RotateDeveloperSecretSchema = z.object({
  client_id: z.string(),
  client_secret: z.string(),
  active_version_id: z.string().uuid(),
  previous_version_id: z.string().uuid(),
  overlap_ends_at: z.string(),
  rotated_at: z.string(),
});

export const DeveloperConsoleWebhookEndpointSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  url: z.string(),
  status: z.enum(['active', 'paused', 'revoked']),
  failed_delivery_count: z.number().int().nonnegative(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const DeveloperWebhookEndpointsSchema = z.object({
  webhooks: z.array(DeveloperConsoleWebhookEndpointSchema),
});

export const DeveloperWebhookDeliverySchema = z.object({
  id: z.string().uuid(),
  endpoint_id: z.string().uuid(),
  event_id: z.string().uuid(),
  event_type: z.string(),
  status: z.enum(['pending', 'delivered', 'failed', 'replayed']),
  attempt_count: z.number().int().nonnegative(),
  response_status: z.number().int().nullable(),
  error_message: z.string().nullable(),
  created_at: z.string(),
  delivered_at: z.string().nullable(),
  replayed_from_delivery_id: z.string().uuid().nullable(),
});

export const DeveloperWebhookDeliveriesSchema = z.object({
  deliveries: z.array(DeveloperWebhookDeliverySchema),
});

export const DeveloperConsoleLogEntrySchema = z.object({
  id: z.string().uuid(),
  source: z.string(),
  event_type: z.string(),
  severity: z.string(),
  message: z.string(),
  created_at: z.string(),
});

export const DeveloperLogsSchema = z.object({
  logs: z.array(DeveloperConsoleLogEntrySchema),
});

export const DeveloperTokenClaimsSchema = z.object({
  subject: z.string(),
  tenant_id: z.string().uuid().nullable(),
  workspace_id: z.string().uuid().nullable(),
  client_id: z.string().nullable(),
  scopes: z.array(z.string()),
  audience: z.string(),
  issuer: z.string(),
  expires_at: z.string(),
  issued_at: z.string(),
  not_before: z.string(),
  token_type: z.string(),
  amr: z.array(z.string()),
  acr: z.string().nullable(),
});

export const DebugDeveloperTokenSchema = z.object({
  active: z.boolean(),
  access_decision: z.enum(['allowed', 'expired', 'tenant_mismatch', 'invalid']),
  claims: DeveloperTokenClaimsSchema.nullable(),
  token_hash_prefix: z.string(),
});

export const DeveloperSandboxTenantSchema = z.object({
  tenant_id: z.string().uuid(),
  sandbox_tenant_id: z.string().uuid(),
  sandbox_name: z.string(),
  sandbox_slug: z.string(),
  status: z.string(),
  data_profile: z.string(),
  reset_requested_at: z.string().nullable(),
  updated_at: z.string(),
});

export const DeveloperSandboxSchema = z.object({
  sandbox: DeveloperSandboxTenantSchema.nullable(),
});

export const DeveloperHealthCheckSchema = z.object({
  id: z.string().uuid(),
  target_type: z.string(),
  target_id: z.string(),
  check_kind: z.string(),
  status: DeveloperHealthStatusSchema,
  summary: z.string(),
  checked_at: z.string(),
});

export const DeveloperHealthChecksSchema = z.object({
  checks: z.array(DeveloperHealthCheckSchema),
});

export type DeveloperPermission = z.infer<typeof DeveloperPermissionSchema>;
export type DeveloperRole = z.infer<typeof DeveloperRoleSchema>;
export type DeveloperMe = z.infer<typeof DeveloperMeSchema>;
export type DeveloperApp = z.infer<typeof DeveloperAppSchema>;
export type PortalDeveloperWebhookEndpoint = z.infer<typeof PortalDeveloperWebhookEndpointSchema>;
export type PortalDeveloperLogEntry = z.infer<typeof PortalDeveloperLogEntrySchema>;
export type DeveloperContext = z.infer<typeof DeveloperContextSchema>;
export type DeveloperOverview = z.infer<typeof DeveloperOverviewSchema>;
export type DeveloperOAuthClient = z.infer<typeof DeveloperOAuthClientSchema>;
export type DeveloperMarketplaceApp = z.infer<typeof DeveloperMarketplaceAppSchema>;
export type DeveloperScopeRegistryEntry = z.infer<typeof DeveloperScopeRegistryEntrySchema>;
export type DeveloperConsentScreen = z.infer<typeof DeveloperConsentScreenSchema>;
export type DeveloperServiceAccount = z.infer<typeof DeveloperServiceAccountSchema>;
export type DeveloperSecretVersion = z.infer<typeof DeveloperSecretVersionSchema>;
export type RotateDeveloperSecret = z.infer<typeof RotateDeveloperSecretSchema>;
export type DeveloperWebhookEndpoint = z.infer<typeof DeveloperConsoleWebhookEndpointSchema>;
export type DeveloperWebhookDelivery = z.infer<typeof DeveloperWebhookDeliverySchema>;
export type DeveloperLogEntry = z.infer<typeof DeveloperConsoleLogEntrySchema>;
export type DeveloperTokenClaims = z.infer<typeof DeveloperTokenClaimsSchema>;
export type DebugDeveloperToken = z.infer<typeof DebugDeveloperTokenSchema>;
export type DeveloperSandboxTenant = z.infer<typeof DeveloperSandboxTenantSchema>;
export type DeveloperHealthCheck = z.infer<typeof DeveloperHealthCheckSchema>;
