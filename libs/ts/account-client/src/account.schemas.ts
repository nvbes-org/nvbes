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
  firstname: z.string().optional(),
  lastname: z.string().optional(),
  username: z.string().max(100).optional(),
  birthdate: z.string().optional(),
  region: z.string().optional(),
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

export const AccountSuccessSchema = z.object({
  success: z.literal(true),
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
