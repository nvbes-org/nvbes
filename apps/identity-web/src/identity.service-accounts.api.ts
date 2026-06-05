import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const NullableStringSchema = z.string().nullable().optional();

const ServiceAccountClientSchema = z.object({
  id: z.string(),
  client_id: z.string(),
  name: z.string(),
  allowed_scopes: z.array(z.string()),
  allowed_audiences: z.array(z.string()),
  allowed_resources: z.array(z.string()),
  client_assertion_required: z.boolean(),
  client_assertion_public_key_configured: z.boolean(),
  required_acr: z.string(),
  created_at: z.string(),
  revoked_at: z.string().nullable().optional(),
});

const ServiceAccountSchema = z.object({
  principal_id: z.string(),
  tenant_id: z.string(),
  organization_id: NullableStringSchema,
  workspace_id: z.string(),
  name: z.string(),
  description: z.string().nullable().optional(),
  role: z.string(),
  status: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
  oauth_clients: z.array(ServiceAccountClientSchema),
});

const ServiceAccountsResultSchema = z.object({
  service_accounts: z.array(ServiceAccountSchema),
});

const CreateServiceAccountInputSchema = z.object({
  name: z.string().min(1).max(100),
  description: z.string().nullable().optional(),
  role: z.string().nullable().optional(),
});

const UpdateServiceAccountInputSchema = z.object({
  name: z.string().min(1).max(100).nullable().optional(),
  description: z.string().nullable().optional(),
  role: z.string().nullable().optional(),
});

const CreateServiceAccountOAuthClientInputSchema = z.object({
  name: z.string().min(1).max(100),
  allowed_scopes: z.array(z.string()),
  allowed_audiences: z.array(z.string()).optional(),
  allowed_resources: z.array(z.string()).optional(),
  client_assertion_public_key_jwk: z.unknown().optional(),
  client_assertion_required: z.boolean().optional(),
  required_acr: z.string().nullable().optional(),
});

const AttachOAuthClientInputSchema = z.object({
  client_id: z.string().min(1),
});

const RotateOAuthClientSecretResultSchema = z.object({
  client_id: z.string(),
  client_secret: z.string(),
  rotated_at: z.string(),
});

export type ServiceAccountClient = z.infer<typeof ServiceAccountClientSchema>;
export type ServiceAccount = z.infer<typeof ServiceAccountSchema>;
export type ServiceAccountsResult = z.infer<typeof ServiceAccountsResultSchema>;
export type CreateServiceAccountInput = z.infer<typeof CreateServiceAccountInputSchema>;
export type UpdateServiceAccountInput = z.infer<typeof UpdateServiceAccountInputSchema>;
export type CreateServiceAccountOAuthClientInput = z.infer<
  typeof CreateServiceAccountOAuthClientInputSchema
>;
export type AttachOAuthClientInput = z.infer<typeof AttachOAuthClientInputSchema>;
export type RotateOAuthClientSecretResult = z.infer<typeof RotateOAuthClientSecretResultSchema>;

type RequestOptions = { signal?: AbortSignal };

export function listServiceAccounts(workspaceId: string, options?: RequestOptions) {
  return identityHttpClient
    .get(`/workspaces/${workspaceId}/service-accounts`, ServiceAccountsResultSchema, options)
    .then((response) => response.service_accounts);
}

export function getServiceAccount(workspaceId: string, serviceAccountId: string) {
  return identityHttpClient.get(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}`,
    ServiceAccountSchema,
  );
}

export function createServiceAccount(workspaceId: string, input: CreateServiceAccountInput) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts`,
    ServiceAccountSchema,
    CreateServiceAccountInputSchema.parse(input),
  );
}

export function updateServiceAccount(
  workspaceId: string,
  serviceAccountId: string,
  input: UpdateServiceAccountInput,
) {
  return identityHttpClient.request(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}`,
    ServiceAccountSchema,
    {
      method: 'PATCH',
      body: UpdateServiceAccountInputSchema.parse(input),
    },
  );
}

export function suspendServiceAccount(workspaceId: string, serviceAccountId: string) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/suspend`,
    ServiceAccountSchema,
    {},
  );
}

export function reactivateServiceAccount(workspaceId: string, serviceAccountId: string) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/reactivate`,
    ServiceAccountSchema,
    {},
  );
}

export function createServiceAccountOAuthClient(
  workspaceId: string,
  serviceAccountId: string,
  input: CreateServiceAccountOAuthClientInput,
) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients`,
    z.object({
      client: ServiceAccountClientSchema,
      client_secret: z.string(),
    }),
    CreateServiceAccountOAuthClientInputSchema.parse(input),
  );
}

export function attachOAuthClient(
  workspaceId: string,
  serviceAccountId: string,
  input: AttachOAuthClientInput,
) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients:attach`,
    ServiceAccountSchema,
    AttachOAuthClientInputSchema.parse(input),
  );
}

export function revokeServiceAccountOAuthClient(
  workspaceId: string,
  serviceAccountId: string,
  clientId: string,
) {
  return identityHttpClient.delete(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients/${clientId}`,
    ServiceAccountSchema,
  );
}

export function rotateServiceAccountOAuthClientSecret(
  workspaceId: string,
  serviceAccountId: string,
  clientId: string,
) {
  return identityHttpClient.post(
    `/workspaces/${workspaceId}/service-accounts/${serviceAccountId}/oauth-clients/${clientId}/rotate-secret`,
    RotateOAuthClientSecretResultSchema,
    {},
  );
}
