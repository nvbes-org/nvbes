import { z } from 'zod';
import { EnterpriseRoleSchema } from './enterprise.schemas';

const NullableStringSchema = z.string().nullable();
const UnknownRecordSchema = z.record(z.string(), z.unknown());

const AccessReviewCampaignStatusSchema = z.enum(['draft', 'active', 'closed']);
const AccessReviewItemTypeSchema = z.enum(['member', 'role', 'service_account', 'oauth_client']);
const AccessReviewItemDecisionSchema = z.enum(['pending', 'approved', 'revoked', 'changed']);

const AccessReviewCampaignScopeInputSchema = z.object({
  include_members: z.boolean(),
  include_roles: z.boolean(),
  include_service_accounts: z.boolean(),
  include_oauth_clients: z.boolean(),
});

const AccessReviewCampaignSummarySchema = z.object({
  id: z.string(),
  name: z.string(),
  description: NullableStringSchema.optional(),
  status: AccessReviewCampaignStatusSchema,
  starts_at: z.string(),
  due_at: z.string(),
  created_by: z.string(),
  created_at: z.string(),
  closed_at: NullableStringSchema.optional(),
  pending_items: z.number(),
  approved_items: z.number(),
  revoked_items: z.number(),
  changed_items: z.number(),
  due_soon_reminders_sent: z.number(),
  overdue_reminders_sent: z.number(),
  last_reminder_at: NullableStringSchema.optional(),
});

export const AccessReviewScheduleSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: NullableStringSchema.optional(),
  recurrence_days: z.number().int(),
  due_after_days: z.number().int(),
  next_run_at: z.string(),
  last_campaign_id: NullableStringSchema.optional(),
  created_by: z.string(),
  created_at: z.string(),
  disabled_at: NullableStringSchema.optional(),
  scope: AccessReviewCampaignScopeInputSchema,
});

const AccessReviewItemSchema = z.object({
  id: z.string(),
  item_type: AccessReviewItemTypeSchema,
  subject_id: z.string(),
  subject_label: z.string(),
  workspace_id: NullableStringSchema.optional(),
  role: NullableStringSchema.optional(),
  status: z.string(),
  evidence: UnknownRecordSchema,
  decision: AccessReviewItemDecisionSchema,
  reviewed_by: NullableStringSchema.optional(),
  reviewed_at: NullableStringSchema.optional(),
  created_at: z.string(),
});

const AccessReviewCampaignExportRowSchema = z.object({
  item_id: z.string(),
  item_type: AccessReviewItemTypeSchema,
  subject_id: z.string(),
  subject_label: z.string(),
  workspace_id: NullableStringSchema.optional(),
  role: NullableStringSchema.optional(),
  status: z.string(),
  decision: AccessReviewItemDecisionSchema,
  reviewed_by: NullableStringSchema.optional(),
  reviewed_at: NullableStringSchema.optional(),
  created_at: z.string(),
  evidence: UnknownRecordSchema,
});

export const AccessReviewCampaignsResponseSchema = z.object({
  campaigns: z.array(AccessReviewCampaignSummarySchema),
});

export const AccessReviewSchedulesResponseSchema = z.object({
  schedules: z.array(AccessReviewScheduleSchema),
});

export const AccessReviewCampaignDetailSchema = z.object({
  campaign: AccessReviewCampaignSummarySchema,
  items: z.array(AccessReviewItemSchema),
});

export const AccessReviewCampaignExportSchema = z.object({
  campaign: AccessReviewCampaignSummarySchema,
  generated_at: z.string(),
  rows: z.array(AccessReviewCampaignExportRowSchema),
});

export const AccessReviewDecisionResponseSchema = z.object({
  campaign: AccessReviewCampaignSummarySchema,
  item: AccessReviewItemSchema,
});

export const CreateAccessReviewCampaignInputSchema = z.object({
  name: z.string().trim().min(1),
  description: z.string().trim().min(1).optional(),
  due_at: z.string().datetime(),
  scope: AccessReviewCampaignScopeInputSchema.refine(hasAccessReviewScope),
});

export const CreateAccessReviewScheduleInputSchema = z
  .object({
    name: z.string().trim().min(1),
    description: z.string().trim().min(1).optional(),
    recurrence_days: z.number().int().min(7).max(366),
    due_after_days: z.number().int().min(1),
    scope: AccessReviewCampaignScopeInputSchema.refine(hasAccessReviewScope),
  })
  .refine((input) => input.due_after_days <= input.recurrence_days, {
    message: 'Due window must not exceed the recurrence.',
    path: ['due_after_days'],
  });

export const AccessReviewDecisionInputSchema = z.object({
  decision: AccessReviewItemDecisionSchema.exclude(['pending']),
  note: z.string().trim().min(1).max(1000).optional(),
  change: z
    .object({
      target_role: EnterpriseRoleSchema.optional(),
    })
    .optional(),
});

export const CloseAccessReviewCampaignInputSchema = z.object({
  note: z.string().trim().min(1).max(1000).optional(),
});

function hasAccessReviewScope(scope: z.infer<typeof AccessReviewCampaignScopeInputSchema>) {
  return (
    scope.include_members ||
    scope.include_roles ||
    scope.include_service_accounts ||
    scope.include_oauth_clients
  );
}

export type AccessReviewCampaignStatus = z.infer<typeof AccessReviewCampaignStatusSchema>;
export type AccessReviewItemType = z.infer<typeof AccessReviewItemTypeSchema>;
export type AccessReviewItemDecision = z.infer<typeof AccessReviewItemDecisionSchema>;
export type AccessReviewCampaignSummary = z.infer<typeof AccessReviewCampaignSummarySchema>;
export type AccessReviewSchedule = z.infer<typeof AccessReviewScheduleSchema>;
export type AccessReviewItem = z.infer<typeof AccessReviewItemSchema>;
export type AccessReviewCampaignsResponse = z.infer<typeof AccessReviewCampaignsResponseSchema>;
export type AccessReviewSchedulesResponse = z.infer<typeof AccessReviewSchedulesResponseSchema>;
export type AccessReviewCampaignDetail = z.infer<typeof AccessReviewCampaignDetailSchema>;
export type AccessReviewCampaignExport = z.infer<typeof AccessReviewCampaignExportSchema>;
export type AccessReviewCampaignExportRow = z.infer<typeof AccessReviewCampaignExportRowSchema>;
export type AccessReviewDecisionInput = z.infer<typeof AccessReviewDecisionInputSchema>;
export type AccessReviewDecisionResponse = z.infer<typeof AccessReviewDecisionResponseSchema>;
export type CloseAccessReviewCampaignInput = z.infer<typeof CloseAccessReviewCampaignInputSchema>;
export type CreateAccessReviewCampaignInput = z.infer<typeof CreateAccessReviewCampaignInputSchema>;
export type CreateAccessReviewScheduleInput = z.infer<typeof CreateAccessReviewScheduleInputSchema>;
