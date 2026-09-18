import { z } from 'zod';

export const NullableStringSchema = z.string().nullable();

export const AccountWorkspaceSchema = z.object({
  id: z.string(),
  name: z.string(),
  workspace_type: z.string(),
  data_region: z.string().optional(),
  role: z.string(),
  trial_ends_at: NullableStringSchema.optional(),
  plan_code: z.string().optional(),
});

export const AccountPrincipalSchema = z.object({
  id: z.string(),
  email: z.string().email(),
  display_name: z.string(),
  firstname: NullableStringSchema.optional(),
  lastname: NullableStringSchema.optional(),
  username: NullableStringSchema.optional(),
  birthdate: NullableStringSchema.optional(),
  region: NullableStringSchema.optional(),
  email_verified: z.boolean(),
  mfa_enabled: z.boolean(),
  created_at: z.string(),
});

export const AccountMeSchema = z.object({
  user: AccountPrincipalSchema,
  current_tenant_id: NullableStringSchema,
  current_organization_id: NullableStringSchema,
  current_workspace_id: NullableStringSchema,
  current_workspace_region: NullableStringSchema,
});

export const EmailAddressSchema = z.object({
  id: z.string(),
  email: z.string().email(),
  is_primary: z.boolean(),
  verified: z.boolean(),
  verified_at: NullableStringSchema,
  created_at: z.string(),
});

export const EmailAddressesResponseSchema = z.object({
  emails: z.array(EmailAddressSchema),
  primary_min_age_hours: z.number(),
  next_cursor: NullableStringSchema,
  has_more: z.boolean(),
});

export const AddSecondaryEmailResponseSchema = z.object({
  email: EmailAddressSchema,
  verification_resend_available_at: z.string(),
});

export const ResendSecondaryEmailVerificationResponseSchema = z.object({
  email: EmailAddressSchema,
  verification_resend_available_at: z.string(),
});

export const PromoteSecondaryEmailResponseSchema = z.object({
  email: EmailAddressSchema,
  user: AccountPrincipalSchema,
});

export const AccountSessionClientSchema = z.object({
  browser: NullableStringSchema,
  browser_version: z.number().nullable(),
  os: NullableStringSchema,
  os_version: NullableStringSchema,
  device: NullableStringSchema,
  device_type: z.enum(['console', 'desktop', 'mobile', 'tablet', 'unknown', 'wearable']),
});

export const AccountSessionSchema = z.object({
  id: z.string(),
  tenant_id: NullableStringSchema,
  organization_id: NullableStringSchema,
  workspace_id: NullableStringSchema,
  workspace_region: NullableStringSchema,
  created_at: z.string(),
  last_seen_at: z.string(),
  expires_at: z.string(),
  revoked_at: NullableStringSchema,
  ip: NullableStringSchema,
  geo_country_code: NullableStringSchema.optional(),
  user_agent: NullableStringSchema,
  client: AccountSessionClientSchema.nullable(),
  device_id: NullableStringSchema,
  device_trust_level: NullableStringSchema,
  device_trust_score: z.number().nullable(),
  risk_score: z.number().nullable(),
  risk_decision: NullableStringSchema,
  risk_confirmed_at: NullableStringSchema,
  current: z.boolean(),
});

export const SessionsResponseSchema = z.object({
  sessions: z.array(AccountSessionSchema),
  next_cursor: NullableStringSchema,
  has_more: z.boolean(),
});

export const DeviceTrustResultSchema = z.object({
  success: z.boolean(),
  device_id: z.string(),
  trust_level: z.string(),
  trust_score: z.number(),
});

export const AccountEntrySchema = z.object({
  authuser: z.string(),
  status: z.enum(['active', 'expired']).optional(),
  message: NullableStringSchema.optional(),
  user: AccountPrincipalSchema,
  session: AccountSessionSchema,
});

export const WorkspacesResponseSchema = z.object({
  workspaces: z.array(AccountWorkspaceSchema).optional(),
});

export const AccountsResponseSchema = z.object({
  accounts: z.array(AccountEntrySchema).optional(),
});

export const UserConsentSchema = z.object({
  id: z.string(),
  principal_id: z.string(),
  consent_type: z.string(),
  document_version: z.string(),
  ip_address: NullableStringSchema.optional(),
  granted_at: z.string(),
  revoked_at: NullableStringSchema.optional(),
});

export const ConsentHistoryResponseSchema = z.object({
  consents: z.array(UserConsentSchema),
  next_cursor: NullableStringSchema,
  has_more: z.boolean(),
});

export const SuccessSchema = z.object({
  success: z.boolean(),
});

export const EmptySchema = z.undefined();

export const GpcStatusSchema = z.object({
  gpc_enabled: z.boolean(),
  gpc_opt_out_active: z.boolean(),
});

export const OAuthClientSchema = z.object({
  id: z.string(),
  client_id: z.string(),
  name: z.string(),
  redirect_uris: z.array(z.string()),
  created_at: z.string(),
  tenant_id: z.string().nullable().optional(),
  owner_scope_type: z.string(),
  owner_scope_id: z.string(),
  client_type: z.string(),
  backchannel_logout_uri: NullableStringSchema.optional(),
  backchannel_logout_session_required: z.boolean().optional(),
  security_event_receiver_uri: NullableStringSchema.optional(),
});

export const OAuthClientsResponseSchema = z.object({
  clients: z.array(OAuthClientSchema),
});

export const CreateWorkspaceInputSchema = z.object({
  name: z.string().min(1).max(100),
  workspace_type: z.string().optional(),
});

export const CreateWorkspaceResponseSchema = z.object({
  workspace: AccountWorkspaceSchema,
});

export const ForgotPasswordResultSchema = z.object({
  success: z.boolean(),
});

export const ResetPasswordResultSchema = z.object({
  success: z.boolean(),
});

export type AccountWorkspace = z.infer<typeof AccountWorkspaceSchema>;
export type AccountMe = z.infer<typeof AccountMeSchema>;
export type AccountSession = z.infer<typeof AccountSessionSchema>;
export type AccountSessionClient = z.infer<typeof AccountSessionClientSchema>;
export type DeviceTrustResult = z.infer<typeof DeviceTrustResultSchema>;
export type AccountPrincipal = z.infer<typeof AccountPrincipalSchema>;
export type AccountEntry = z.infer<typeof AccountEntrySchema>;
export type EmailAddress = z.infer<typeof EmailAddressSchema>;
export type EmailAddressesResponse = z.infer<typeof EmailAddressesResponseSchema>;
export type AddSecondaryEmailResponse = z.infer<typeof AddSecondaryEmailResponseSchema>;
export type ResendSecondaryEmailVerificationResponse = z.infer<
  typeof ResendSecondaryEmailVerificationResponseSchema
>;
export type PromoteSecondaryEmailResponse = z.infer<typeof PromoteSecondaryEmailResponseSchema>;
export type UserConsent = z.infer<typeof UserConsentSchema>;
export type ConsentHistoryResponse = z.infer<typeof ConsentHistoryResponseSchema>;
export type GpcStatus = z.infer<typeof GpcStatusSchema>;
export type OAuthClient = z.infer<typeof OAuthClientSchema>;
export type OAuthClientsResponse = z.infer<typeof OAuthClientsResponseSchema>;
export type CreateWorkspaceInput = z.infer<typeof CreateWorkspaceInputSchema>;
export type ForgotPasswordResult = z.infer<typeof ForgotPasswordResultSchema>;
export type ResetPasswordResult = z.infer<typeof ResetPasswordResultSchema>;
