import { z } from 'zod';

const NullableStringSchema = z.string().nullable().optional();

export const ServiceAccountClientSchema = z.object({
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

export const ServiceAccountSchema = z.object({
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

export const ServiceAccountsResultSchema = z.object({
  service_accounts: z.array(ServiceAccountSchema),
});

export const CreateServiceAccountInputSchema = z.object({
  name: z.string().min(1).max(100),
  description: z.string().nullable().optional(),
  role: z.string().nullable().optional(),
});

export const UpdateServiceAccountInputSchema = z.object({
  name: z.string().min(1).max(100).nullable().optional(),
  description: z.string().nullable().optional(),
  role: z.string().nullable().optional(),
});

export const CreateServiceAccountOAuthClientInputSchema = z.object({
  name: z.string().min(1).max(100),
  allowed_scopes: z.array(z.string()),
  allowed_audiences: z.array(z.string()).optional(),
  allowed_resources: z.array(z.string()).optional(),
  client_assertion_public_key_jwk: z.unknown().optional(),
  client_assertion_required: z.boolean().optional(),
  required_acr: z.string().nullable().optional(),
});

export const AttachOAuthClientInputSchema = z.object({
  client_id: z.string().min(1),
});

export const CreateServiceAccountOAuthClientResultSchema = z.object({
  client: ServiceAccountClientSchema,
  client_secret: z.string(),
});

export const RotateOAuthClientSecretResultSchema = z.object({
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
export type CreateServiceAccountOAuthClientResult = z.infer<
  typeof CreateServiceAccountOAuthClientResultSchema
>;
export type RotateOAuthClientSecretResult = z.infer<typeof RotateOAuthClientSecretResultSchema>;
