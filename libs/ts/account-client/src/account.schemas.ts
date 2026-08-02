import { z } from 'zod';

const NullableStringSchema = z.string().nullable();

export const AccountProfileSchema = z.object({
  id: z.string(),
  display_name: z.string(),
  firstname: NullableStringSchema,
  lastname: NullableStringSchema,
  username: NullableStringSchema,
  birthdate: NullableStringSchema,
  region: NullableStringSchema,
  created_at: z.string(),
});

export const AccountProfileEnvelopeSchema = z.object({
  user: AccountProfileSchema,
});

export const AccountUpdateProfileInputSchema = z.object({
  firstname: NullableStringSchema,
  lastname: NullableStringSchema,
  username: z.string().max(100).nullable(),
  birthdate: NullableStringSchema,
  region: NullableStringSchema,
});

export const AccountThemeSchema = z.enum(['system', 'light', 'dark']);
export const AccountLanguageSchema = z.enum(['fr', 'en']);

export const AccountPreferencesSchema = z.object({
  theme: AccountThemeSchema,
  language: AccountLanguageSchema,
});

export const AccountNotificationsSchema = z.object({
  email: z.boolean(),
  push: z.boolean(),
  in_app: z.boolean(),
  marketing_email: z.boolean(),
});

export const AccountAvatarUploadInputSchema = z.object({
  content_type: z.enum(['image/jpeg', 'image/png', 'image/webp']),
  size_bytes: z
    .number()
    .int()
    .positive()
    .max(5 * 1024 * 1024),
});

export const AccountAvatarUploadSchema = z.object({
  upload_url: z.string().url(),
  object_key: z.string(),
});

export const AccountConsentSchema = z.object({
  id: z.string(),
  principal_id: z.string(),
  consent_type: z.string(),
  document_version: z.string(),
  ip_address: NullableStringSchema,
  granted_at: z.string(),
  revoked_at: NullableStringSchema,
});

export const AccountConsentInputSchema = z.object({
  consent_type: z.string().min(1),
  document_version: z.string().min(1),
});

export const AccountConsentHistorySchema = z.object({
  consents: z.array(AccountConsentSchema),
  next_cursor: NullableStringSchema,
  has_more: z.boolean(),
});

export const AccountGpcStatusSchema = z.object({
  gpc_enabled: z.boolean(),
  gpc_opt_out_active: z.boolean(),
});

export const AccountSessionClientSchema = z.object({
  browser: NullableStringSchema,
  browser_version: z.number().nullable(),
  os: NullableStringSchema,
  os_version: NullableStringSchema,
  device: NullableStringSchema,
  device_type: z.string(),
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
  geo_country_code: NullableStringSchema,
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

export const AccountSessionsPageSchema = z.object({
  sessions: z.array(AccountSessionSchema),
  next_cursor: NullableStringSchema,
  has_more: z.boolean(),
});

export const AccountSuccessSchema = z.object({
  success: z.literal(true),
});

export const AccountClosureSchema = z.object({
  saga_id: z.uuid(),
  status: z.enum(['pending', 'dispatching', 'completed']),
  requested_at: z.iso.datetime(),
});

export const AccountClosureParticipantSchema = z.object({
  participant: z.enum(['cloud', 'billing', 'identity', 'account']),
  status: z.enum(['pending', 'processing', 'completed', 'failed']),
  attempts: z.number().int().nonnegative(),
  completed_at: z.iso.datetime().nullable(),
  last_error: NullableStringSchema,
});

export const AccountClosureStatusSchema = z.object({
  saga_id: z.uuid(),
  status: z.enum(['pending', 'dispatching', 'completed', 'failed', 'cancelled']),
  requested_at: z.iso.datetime(),
  updated_at: z.iso.datetime(),
  completed_at: z.iso.datetime().nullable(),
  last_error: NullableStringSchema,
  participants: z.array(AccountClosureParticipantSchema),
});

export const EmptyResponseSchema = z.undefined();

export type AccountProfile = z.infer<typeof AccountProfileSchema>;
export type AccountUpdateProfileInput = z.infer<typeof AccountUpdateProfileInputSchema>;
export type AccountTheme = z.infer<typeof AccountThemeSchema>;
export type AccountLanguage = z.infer<typeof AccountLanguageSchema>;
export type AccountPreferences = z.infer<typeof AccountPreferencesSchema>;
export type AccountNotifications = z.infer<typeof AccountNotificationsSchema>;
export type AccountAvatarUploadInput = z.infer<typeof AccountAvatarUploadInputSchema>;
export type AccountAvatarUpload = z.infer<typeof AccountAvatarUploadSchema>;
export type AccountConsent = z.infer<typeof AccountConsentSchema>;
export type AccountConsentInput = z.infer<typeof AccountConsentInputSchema>;
export type AccountConsentHistory = z.infer<typeof AccountConsentHistorySchema>;
export type AccountGpcStatus = z.infer<typeof AccountGpcStatusSchema>;
export type AccountSession = z.infer<typeof AccountSessionSchema>;
export type AccountSessionsPage = z.infer<typeof AccountSessionsPageSchema>;
export type AccountClosure = z.infer<typeof AccountClosureSchema>;
export type AccountClosureParticipant = z.infer<typeof AccountClosureParticipantSchema>;
export type AccountClosureStatus = z.infer<typeof AccountClosureStatusSchema>;
