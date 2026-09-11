import { z } from 'zod';
import { EnterpriseAuditEventSchema } from './enterprise.schemas';

const NullableStringSchema = z.string().nullable();

const EnterpriseTrustCenterTenantSchema = z.object({
  id: z.string(),
  name: z.string(),
  slug: z.string(),
  status: z.string(),
  security_tier: z.string(),
});

const EnterpriseTrustCenterMfaSchema = z.object({
  enabled: z.boolean(),
  active_members: z.number(),
  members_with_mfa: z.number(),
  active_factors: z.number(),
  passkey_factors: z.number(),
});

const EnterpriseTrustCenterSsoProviderSchema = z.object({
  id: z.string(),
  name: z.string(),
  provider_type: z.string(),
  provider_family: z.string(),
  status: z.string(),
  created_at: z.string(),
});

const EnterpriseTrustCenterSsoSchema = z.object({
  enabled: z.boolean(),
  active_providers: z.number(),
  required_domains: z.number(),
  providers: z.array(EnterpriseTrustCenterSsoProviderSchema),
});

const EnterpriseTrustCenterDomainSchema = z.object({
  id: z.string(),
  domain: z.string(),
  verified: z.boolean(),
  sso_required: z.boolean(),
  sso_provider_id: NullableStringSchema.optional(),
  verified_at: NullableStringSchema.optional(),
});

const EnterpriseTrustCenterAuditSchema = z.object({
  immutable: z.boolean(),
  recent_events: z.array(EnterpriseAuditEventSchema),
});

const EnterpriseTrustCenterHostingRegionSchema = z.object({
  data_region: z.string(),
  legal_jurisdiction: z.string(),
  workspace_count: z.number(),
});

const EnterpriseTrustCenterDocumentSchema = z.object({
  name: z.string(),
  status: z.string(),
  version: z.string(),
  url: z.string(),
});

const EnterpriseTrustCenterSubprocessorSchema = z.object({
  name: z.string(),
  service: z.string(),
  data_categories: z.string(),
  location: z.string(),
  transfer_outside_eea: z.boolean(),
  transfer_safeguard: z.string(),
});

export const EnterpriseTrustCenterResponseSchema = z.object({
  tenant: EnterpriseTrustCenterTenantSchema,
  mfa: EnterpriseTrustCenterMfaSchema,
  sso: EnterpriseTrustCenterSsoSchema,
  verified_domains: z.array(EnterpriseTrustCenterDomainSchema),
  audit: EnterpriseTrustCenterAuditSchema,
  hosting_regions: z.array(EnterpriseTrustCenterHostingRegionSchema),
  dpa: EnterpriseTrustCenterDocumentSchema,
  subprocessors: z.array(EnterpriseTrustCenterSubprocessorSchema),
  generated_at: z.string(),
});

export type EnterpriseTrustCenterResponse = z.infer<typeof EnterpriseTrustCenterResponseSchema>;
