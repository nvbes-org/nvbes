import { z } from 'zod';

const NullableStringSchema = z.string().nullable();
const UnknownRecordSchema = z.record(z.string(), z.unknown());

export const EnterpriseRoleSchema = z.enum(['owner', 'admin', 'member', 'viewer']);
export const EnterpriseModuleGrantSchema = z.enum([
  'members',
  'workspaces',
  'developers',
  'policies',
  'security',
  'billing',
  'audit',
  'drive',
]);

const EnterprisePageSchema = z.object({
  cursor: NullableStringSchema,
  has_more: z.boolean(),
});

const EnterpriseUserSchema = z.object({
  id: z.string(),
  email: z.string().email(),
  display_name: z.string(),
  role: EnterpriseRoleSchema,
  module_grants: z.array(EnterpriseModuleGrantSchema),
  workspace_ids: z.array(z.string()),
  status: z.string(),
  mfa_enabled: z.boolean(),
  last_seen_at: NullableStringSchema.optional(),
  created_at: z.string(),
});

const EnterpriseInvitationSchema = z.object({
  id: z.string(),
  email: z.string().email(),
  role: EnterpriseRoleSchema,
  module_grants: z.array(EnterpriseModuleGrantSchema),
  workspace_ids: z.array(z.string()),
  status: z.string(),
  invited_at: z.string(),
  expires_at: NullableStringSchema.optional(),
});

const EnterpriseWorkspaceSchema = z.object({
  id: z.string(),
  name: z.string(),
  workspace_type: z.string(),
  data_region: NullableStringSchema.optional(),
  member_count: z.number(),
  storage_used_bytes: z.number(),
  created_at: z.string(),
});

const EnterpriseDeveloperCredentialSchema = z.object({
  id: z.string(),
  name: z.string(),
  owner_email: z.string().email().optional(),
  scopes: z.array(z.string()),
  last_used_at: NullableStringSchema.optional(),
  created_at: z.string(),
  expires_at: NullableStringSchema.optional(),
});

const EnterprisePolicySchema = z.object({
  id: z.string(),
  name: z.string(),
  category: z.string(),
  enabled: z.boolean(),
  configuration: UnknownRecordSchema,
  updated_at: z.string(),
});

const EnterpriseSecuritySignalSchema = z.object({
  key: z.string(),
  label: z.string(),
  status: z.string(),
  severity: z.string(),
  details: UnknownRecordSchema.optional(),
});

const EnterpriseAuditEventSchema = z.object({
  id: z.string(),
  event_type: z.string(),
  actor_id: NullableStringSchema.optional(),
  actor_email: NullableStringSchema.optional(),
  target_type: NullableStringSchema.optional(),
  target_id: NullableStringSchema.optional(),
  metadata: UnknownRecordSchema.optional(),
  created_at: z.string(),
});

const EnterpriseBillingPlanSchema = z.object({
  code: z.string(),
  name: z.string(),
  status: z.string(),
  currency: z.string(),
  monthly_price_cents: z.number(),
});

const EnterpriseInvoiceSchema = z.object({
  id: z.string(),
  status: z.string(),
  amount_due_cents: z.number(),
  currency: z.string(),
  issued_at: z.string(),
  hosted_invoice_url: NullableStringSchema.optional(),
});

const EnterpriseUsageMetricSchema = z.object({
  key: z.string(),
  label: z.string(),
  value: z.number(),
  limit: z.number().nullable().optional(),
  unit: z.string(),
});

const EnterpriseOverviewMetricSchema = z.object({
  key: z.string(),
  label: z.string(),
  value: z.number(),
  delta_percent: z.number().nullable().optional(),
});

export const EnterpriseContextResponseSchema = z.object({
  tenant_id: z.string(),
  organization_id: NullableStringSchema.optional(),
  workspace_id: NullableStringSchema.optional(),
  user_id: z.string(),
  role: EnterpriseRoleSchema,
  module_grants: z.array(EnterpriseModuleGrantSchema),
  available_roles: z.array(EnterpriseRoleSchema),
  available_module_grants: z.array(EnterpriseModuleGrantSchema),
});

export const EnterpriseOverviewResponseSchema = z.object({
  metrics: z.array(EnterpriseOverviewMetricSchema),
  security_signals: z.array(EnterpriseSecuritySignalSchema),
  recent_audit_events: z.array(EnterpriseAuditEventSchema),
});

export const EnterpriseUsersResponseSchema = z.object({
  users: z.array(EnterpriseUserSchema),
  invitations: z.array(EnterpriseInvitationSchema),
  roles: z.array(EnterpriseRoleSchema),
  module_grants: z.array(EnterpriseModuleGrantSchema),
  page: EnterprisePageSchema,
});

export const EnterpriseInvitationsResponseSchema = z.object({
  invitations: z.array(EnterpriseInvitationSchema),
});

export const EnterpriseAccessUpdateResponseSchema = z.object({
  user: EnterpriseUserSchema,
});

export const EnterpriseWorkspacesResponseSchema = z.object({
  workspaces: z.array(EnterpriseWorkspaceSchema),
  page: EnterprisePageSchema.optional(),
});

export const EnterpriseDevelopersResponseSchema = z.object({
  credentials: z.array(EnterpriseDeveloperCredentialSchema),
  page: EnterprisePageSchema.optional(),
});

export const EnterprisePoliciesResponseSchema = z.object({
  policies: z.array(EnterprisePolicySchema),
});

export const EnterpriseSecurityResponseSchema = z.object({
  signals: z.array(EnterpriseSecuritySignalSchema),
  mfa_required: z.boolean(),
  passkeys_enabled: z.boolean(),
  recovery_approval_required: z.boolean(),
});

export const EnterpriseAuditEventsResponseSchema = z.object({
  events: z.array(EnterpriseAuditEventSchema),
  page: EnterprisePageSchema,
});

export const EnterpriseBillingResponseSchema = z.object({
  plan: EnterpriseBillingPlanSchema,
  invoices: z.array(EnterpriseInvoiceSchema),
  billing_email: NullableStringSchema.optional(),
});

export const EnterpriseUsageResponseSchema = z.object({
  metrics: z.array(EnterpriseUsageMetricSchema),
});

export const EnterpriseInvitationInputSchema = z.object({
  emails: z.array(z.string().email()).min(1),
  role: EnterpriseRoleSchema,
  module_grants: z.array(EnterpriseModuleGrantSchema),
  workspace_ids: z.array(z.string()),
});

export const EnterpriseAccessUpdateInputSchema = z.object({
  role: EnterpriseRoleSchema,
  module_grants: z.array(EnterpriseModuleGrantSchema),
  workspace_ids: z.array(z.string()),
});

const AuditReasonSchema = z.string().refine((value) => value.trim().length > 0);

export const EnterpriseSuspendInputSchema = z.object({
  reason: AuditReasonSchema,
});

export const EnterpriseReactivateInputSchema = z.object({
  reason: AuditReasonSchema,
  module_grants: z.array(EnterpriseModuleGrantSchema).optional(),
  workspace_ids: z.array(z.string()).optional(),
});

export type EnterpriseRole = z.infer<typeof EnterpriseRoleSchema>;
export type EnterpriseModuleGrant = z.infer<typeof EnterpriseModuleGrantSchema>;
export type EnterpriseContextResponse = z.infer<typeof EnterpriseContextResponseSchema>;
export type EnterpriseOverviewResponse = z.infer<typeof EnterpriseOverviewResponseSchema>;
export type EnterpriseUsersResponse = z.infer<typeof EnterpriseUsersResponseSchema>;
export type EnterpriseInvitationsResponse = z.infer<typeof EnterpriseInvitationsResponseSchema>;
export type EnterpriseAccessUpdateResponse = z.infer<typeof EnterpriseAccessUpdateResponseSchema>;
export type EnterpriseWorkspacesResponse = z.infer<typeof EnterpriseWorkspacesResponseSchema>;
export type EnterpriseDevelopersResponse = z.infer<typeof EnterpriseDevelopersResponseSchema>;
export type EnterprisePoliciesResponse = z.infer<typeof EnterprisePoliciesResponseSchema>;
export type EnterpriseSecurityResponse = z.infer<typeof EnterpriseSecurityResponseSchema>;
export type EnterpriseAuditEventsResponse = z.infer<typeof EnterpriseAuditEventsResponseSchema>;
export type EnterpriseBillingResponse = z.infer<typeof EnterpriseBillingResponseSchema>;
export type EnterpriseUsageResponse = z.infer<typeof EnterpriseUsageResponseSchema>;
export type EnterpriseInvitationInput = z.infer<typeof EnterpriseInvitationInputSchema>;
export type EnterpriseAccessUpdateInput = z.infer<typeof EnterpriseAccessUpdateInputSchema>;
export type EnterpriseSuspendInput = z.infer<typeof EnterpriseSuspendInputSchema>;
export type EnterpriseReactivateInput = z.infer<typeof EnterpriseReactivateInputSchema>;
