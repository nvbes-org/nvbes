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

export type TenantDomain = z.infer<typeof TenantDomainSchema>;
export type TenantDomainsResponse = z.infer<typeof TenantDomainsResponseSchema>;
export type TenantDomainResponse = z.infer<typeof TenantDomainResponseSchema>;
export type CreateTenantDomainInput = z.infer<typeof CreateTenantDomainInputSchema>;
export type UpdateTenantDomainInput = z.infer<typeof UpdateTenantDomainInputSchema>;
export type VerifyTenantDomainInput = z.infer<typeof VerifyTenantDomainInputSchema>;
