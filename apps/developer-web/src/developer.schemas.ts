import { z } from 'zod';

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
]);

export const DeveloperRoleSchema = z.enum([
  'developer_admin',
  'app_manager',
  'webhook_manager',
  'log_viewer',
  'integration_tester',
  'docs_viewer',
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

export const DeveloperWebhookEndpointSchema = z.object({
  id: z.string(),
  name: z.string(),
  url: z.string(),
  status: z.string(),
  events: z.array(
    z.enum(['user.created', 'login.failed', 'session.revoked', 'client.created']),
  ),
  signing_secret_last4: z.string(),
  created_at: z.string(),
});

export const DeveloperWebhooksResponseSchema = z.object({
  endpoints: z.array(DeveloperWebhookEndpointSchema),
});

export const CreateDeveloperWebhookEndpointResponseSchema = z.object({
  endpoint: DeveloperWebhookEndpointSchema,
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

export const DeveloperLogEntrySchema = z.object({
  id: z.string(),
  event_type: z.string(),
  user_id: z.string().nullable(),
  client_id: z.string().nullable(),
  tenant_id: z.string().nullable(),
  created_at: z.string(),
});

export const DeveloperLogsResponseSchema = z.object({
  logs: z.array(DeveloperLogEntrySchema),
});

export type DeveloperPermission = z.infer<typeof DeveloperPermissionSchema>;
export type DeveloperRole = z.infer<typeof DeveloperRoleSchema>;
export type DeveloperMe = z.infer<typeof DeveloperMeSchema>;
export type DeveloperApp = z.infer<typeof DeveloperAppSchema>;
export type DeveloperWebhookEndpoint = z.infer<typeof DeveloperWebhookEndpointSchema>;
export type DeveloperLogEntry = z.infer<typeof DeveloperLogEntrySchema>;
