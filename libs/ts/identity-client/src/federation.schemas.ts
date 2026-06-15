import { z } from 'zod';

const NullableStringSchema = z.string().nullable();

export const TenantDomainSchema = z.object({
  id: z.string(),
  domain: z.string(),
  sso_required: z.boolean(),
  sso_provider_id: NullableStringSchema.optional(),
  verified_at: NullableStringSchema.optional(),
  verification_requested_at: NullableStringSchema.optional(),
  verification_expires_at: NullableStringSchema.optional(),
  created_at: z.string(),
});

export const TenantDomainsResponseSchema = z.object({
  domains: z.array(TenantDomainSchema),
});

export const TenantDomainResponseSchema = z.object({
  domain: TenantDomainSchema,
  verification_token: NullableStringSchema.optional(),
});

export const CreateTenantDomainInputSchema = z.object({
  domain: z.string().trim().min(1),
  sso_required: z.boolean().optional(),
  sso_provider_id: z.string().optional(),
});

export const UpdateTenantDomainInputSchema = z.object({
  sso_required: z.boolean().optional(),
  sso_provider_id: z.string().nullable().optional(),
});

export const VerifyTenantDomainInputSchema = z.object({
  token: z.string().trim().min(1),
});

export const FederatedIdentityProviderSchema = z.object({
  id: z.string(),
  provider_type: z.string(),
  provider_family: z.string(),
  name: z.string(),
  client_id: NullableStringSchema.optional(),
  issuer: NullableStringSchema.optional(),
  metadata_url: NullableStringSchema.optional(),
  status: z.string(),
  sp_entity_id: NullableStringSchema.optional(),
  attribute_mapping: z.unknown(),
  encryption_cert_pem: NullableStringSchema.optional(),
  require_signed_assertions: z.boolean(),
  require_signed_responses: z.boolean(),
  created_at: z.string(),
});

export const FederatedIdentityProviderResponseSchema = z.object({
  provider: FederatedIdentityProviderSchema,
});

export const FederatedIdentityProvidersResponseSchema = z.object({
  providers: z.array(FederatedIdentityProviderSchema),
});

export const CreateFederatedIdentityProviderInputSchema = z.object({
  provider_type: z.string().trim().min(1),
  provider_family: z.string().trim().min(1).optional(),
  name: z.string().trim().min(1),
  client_id: z.string().trim().min(1).optional(),
  issuer: z.string().trim().min(1).optional(),
  metadata_url: z.string().trim().min(1).optional(),
  status: z.string().trim().min(1).optional(),
});

export type TenantDomain = z.infer<typeof TenantDomainSchema>;
export type TenantDomainsResponse = z.infer<typeof TenantDomainsResponseSchema>;
export type TenantDomainResponse = z.infer<typeof TenantDomainResponseSchema>;
export type CreateTenantDomainInput = z.infer<typeof CreateTenantDomainInputSchema>;
export type UpdateTenantDomainInput = z.infer<typeof UpdateTenantDomainInputSchema>;
export type VerifyTenantDomainInput = z.infer<typeof VerifyTenantDomainInputSchema>;
export type FederatedIdentityProvider = z.infer<typeof FederatedIdentityProviderSchema>;
export type FederatedIdentityProviderResponse = z.infer<
  typeof FederatedIdentityProviderResponseSchema
>;
export type FederatedIdentityProvidersResponse = z.infer<
  typeof FederatedIdentityProvidersResponseSchema
>;
export type CreateFederatedIdentityProviderInput = z.infer<
  typeof CreateFederatedIdentityProviderInputSchema
>;
