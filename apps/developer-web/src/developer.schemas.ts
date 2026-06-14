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

export type DeveloperPermission = z.infer<typeof DeveloperPermissionSchema>;
export type DeveloperRole = z.infer<typeof DeveloperRoleSchema>;
export type DeveloperMe = z.infer<typeof DeveloperMeSchema>;
export type DeveloperApp = z.infer<typeof DeveloperAppSchema>;
export type DeveloperWebhookEndpoint = z.infer<typeof DeveloperWebhookEndpointSchema>;
