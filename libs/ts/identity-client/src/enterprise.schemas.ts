import { z } from "zod";

const NullableStringSchema = z.string().nullable();
const UnknownRecordSchema = z.record(z.string(), z.unknown());

export const EnterpriseRoleSchema = z.enum([
	"owner",
	"admin",
	"member",
	"viewer",
]);
export const EnterpriseModuleGrantSchema = z.enum([
	"members",
	"workspaces",
	"developers",
	"policies",
	"security",
	"billing",
	"audit",
	"drive",
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

export const EnterpriseAuditEventSchema = z.object({
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

const AccessReviewCampaignStatusSchema = z.enum(["draft", "active", "closed"]);
const AccessReviewItemTypeSchema = z.enum([
	"member",
	"role",
	"service_account",
	"oauth_client",
]);
const AccessReviewItemDecisionSchema = z.enum([
	"pending",
	"approved",
	"revoked",
	"changed",
]);

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

export const EnterprisePolicySimulationSubjectSchema = z.discriminatedUnion(
	"subject_type",
	[
		z.object({
			subject_type: z.literal("user"),
			user_id: z.string().uuid(),
		}),
		z.object({
			subject_type: z.literal("client"),
			client_id: z.string().min(1),
		}),
	],
);

export const EnterprisePolicySimulationInputSchema = z.object({
	workspace_id: z.string().uuid(),
	subject: EnterprisePolicySimulationSubjectSchema,
	action: z.string().min(1),
	resource: z
		.object({
			owns_resource: z.boolean().optional(),
			member_share_links_enabled: z.boolean().optional(),
			target_role: z.string().optional(),
		})
		.optional(),
});

const EnterprisePolicySimulationDecisionSchema = z.object({
	allowed: z.boolean(),
	reason: z.string(),
	action: z.string(),
	workspace_id: z.string(),
	subject_type: z.enum(["user", "client"]),
	subject_id: z.string(),
	subject_label: z.string(),
	role: NullableStringSchema.optional(),
	requires_step_up: z.boolean(),
});

export const EnterprisePolicySimulationResponseSchema = z.object({
	decision: EnterprisePolicySimulationDecisionSchema,
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

export const AccessReviewCampaignsResponseSchema = z.object({
	campaigns: z.array(AccessReviewCampaignSummarySchema),
});

export const AccessReviewCampaignDetailSchema = z.object({
	campaign: AccessReviewCampaignSummarySchema,
	items: z.array(AccessReviewItemSchema),
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

export const CreateAccessReviewCampaignInputSchema = z.object({
	name: z.string().trim().min(1),
	description: z.string().trim().min(1).optional(),
	due_at: z.string().datetime(),
	scope: AccessReviewCampaignScopeInputSchema.refine(
		(scope) =>
			scope.include_members ||
			scope.include_roles ||
			scope.include_service_accounts ||
			scope.include_oauth_clients,
	),
});

export type EnterpriseRole = z.infer<typeof EnterpriseRoleSchema>;
export type EnterpriseModuleGrant = z.infer<typeof EnterpriseModuleGrantSchema>;
export type AccessReviewCampaignStatus = z.infer<
	typeof AccessReviewCampaignStatusSchema
>;
export type AccessReviewItemType = z.infer<typeof AccessReviewItemTypeSchema>;
export type AccessReviewItemDecision = z.infer<
	typeof AccessReviewItemDecisionSchema
>;
export type AccessReviewCampaignSummary = z.infer<
	typeof AccessReviewCampaignSummarySchema
>;
export type AccessReviewItem = z.infer<typeof AccessReviewItemSchema>;
export type AccessReviewCampaignsResponse = z.infer<
	typeof AccessReviewCampaignsResponseSchema
>;
export type AccessReviewCampaignDetail = z.infer<
	typeof AccessReviewCampaignDetailSchema
>;
export type CreateAccessReviewCampaignInput = z.infer<
	typeof CreateAccessReviewCampaignInputSchema
>;
export type EnterpriseContextResponse = z.infer<
	typeof EnterpriseContextResponseSchema
>;
export type EnterpriseOverviewResponse = z.infer<
	typeof EnterpriseOverviewResponseSchema
>;
export type EnterpriseUsersResponse = z.infer<
	typeof EnterpriseUsersResponseSchema
>;
export type EnterpriseInvitationsResponse = z.infer<
	typeof EnterpriseInvitationsResponseSchema
>;
export type EnterpriseAccessUpdateResponse = z.infer<
	typeof EnterpriseAccessUpdateResponseSchema
>;
export type EnterpriseWorkspacesResponse = z.infer<
	typeof EnterpriseWorkspacesResponseSchema
>;
export type EnterpriseDevelopersResponse = z.infer<
	typeof EnterpriseDevelopersResponseSchema
>;
export type EnterprisePoliciesResponse = z.infer<
	typeof EnterprisePoliciesResponseSchema
>;
export type EnterprisePolicySimulationSubject = z.infer<
	typeof EnterprisePolicySimulationSubjectSchema
>;
export type EnterprisePolicySimulationInput = z.infer<
	typeof EnterprisePolicySimulationInputSchema
>;
export type EnterprisePolicySimulationResponse = z.infer<
	typeof EnterprisePolicySimulationResponseSchema
>;
export type EnterpriseSecurityResponse = z.infer<
	typeof EnterpriseSecurityResponseSchema
>;
export type EnterpriseAuditEventsResponse = z.infer<
	typeof EnterpriseAuditEventsResponseSchema
>;
export type EnterpriseBillingResponse = z.infer<
	typeof EnterpriseBillingResponseSchema
>;
export type EnterpriseUsageResponse = z.infer<
	typeof EnterpriseUsageResponseSchema
>;
export type EnterpriseInvitationInput = z.infer<
	typeof EnterpriseInvitationInputSchema
>;
export type EnterpriseAccessUpdateInput = z.infer<
	typeof EnterpriseAccessUpdateInputSchema
>;
export type EnterpriseSuspendInput = z.infer<
	typeof EnterpriseSuspendInputSchema
>;
export type EnterpriseReactivateInput = z.infer<
	typeof EnterpriseReactivateInputSchema
>;
