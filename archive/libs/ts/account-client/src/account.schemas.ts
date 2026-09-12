import { z } from 'zod';

const NullableStringSchema = z.string().nullable();
const TimestampSchema = z.iso.datetime();

export const AccountProfileSchema = z.object({
  id: z.uuid(),
  display_name: z.string(),
  firstname: NullableStringSchema,
  lastname: NullableStringSchema,
  username: NullableStringSchema,
  birthdate: NullableStringSchema,
  region: NullableStringSchema,
  created_at: TimestampSchema,
});
export const AccountProfileEnvelopeSchema = z.object({ user: AccountProfileSchema });
export const AccountUpdateProfileInputSchema = z.object({
  firstname: z.string().max(100).nullable(),
  lastname: z.string().max(100).nullable(),
  username: z.string().min(3).max(100).nullable(),
  birthdate: NullableStringSchema,
  region: z.string().min(2).max(32).nullable(),
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

export const AccountTeamSchema = z.object({
  id: z.uuid(),
  name: z.string(),
  role: z.enum(['owner', 'member']),
  created_at: TimestampSchema,
});
export const AccountTeamsEnvelopeSchema = z.object({ teams: z.array(AccountTeamSchema) });
export const AccountCreateTeamInputSchema = z.object({ name: z.string().min(1).max(100) });
export const AccountCreatedTeamSchema = AccountTeamSchema.extend({ join_code: z.string() });
export const AccountJoinTeamInputSchema = z.object({ join_code: z.string() });

export const AccountExportRequestSchema = z.object({
  export_id: z.uuid(),
  status: z.enum(['pending', 'processing']),
  requested_at: TimestampSchema,
});
const AccountParticipantSchema = z.object({
  participant: z.enum([
    'identity',
    'account',
    'billing',
    'email',
    'trust_risk',
    'platform_operations',
  ]),
  status: z.enum(['pending', 'processing', 'completed', 'failed', 'cancelled', 'expired']),
  attempts: z.number().int().nonnegative(),
  completed_at: TimestampSchema.nullable(),
  last_error: NullableStringSchema,
});
export const AccountExportParticipantSchema = AccountParticipantSchema;
export const AccountExportStatusSchema = z.object({
  export_id: z.uuid(),
  status: z.enum(['pending', 'processing', 'completed', 'failed', 'expired']),
  requested_at: TimestampSchema,
  updated_at: TimestampSchema,
  completed_at: TimestampSchema.nullable(),
  expires_at: TimestampSchema.nullable(),
  last_error: NullableStringSchema,
  participants: z.array(AccountExportParticipantSchema),
});

export const AccountClosureSchema = z.object({
  saga_id: z.uuid(),
  status: z.enum(['pending', 'processing']),
  requested_at: TimestampSchema,
});
export const AccountClosureParticipantSchema = AccountParticipantSchema;
export const AccountClosureStatusSchema = z.object({
  saga_id: z.uuid(),
  status: z.enum(['pending', 'processing', 'completed', 'failed', 'cancelled']),
  requested_at: TimestampSchema,
  updated_at: TimestampSchema,
  completed_at: TimestampSchema.nullable(),
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
export type AccountTeam = z.infer<typeof AccountTeamSchema>;
export type AccountCreateTeamInput = z.infer<typeof AccountCreateTeamInputSchema>;
export type AccountCreatedTeam = z.infer<typeof AccountCreatedTeamSchema>;
export type AccountJoinTeamInput = z.infer<typeof AccountJoinTeamInputSchema>;
export type AccountExportRequest = z.infer<typeof AccountExportRequestSchema>;
export type AccountExportParticipant = z.infer<typeof AccountExportParticipantSchema>;
export type AccountExportStatus = z.infer<typeof AccountExportStatusSchema>;
export type AccountClosure = z.infer<typeof AccountClosureSchema>;
export type AccountClosureParticipant = z.infer<typeof AccountClosureParticipantSchema>;
export type AccountClosureStatus = z.infer<typeof AccountClosureStatusSchema>;
