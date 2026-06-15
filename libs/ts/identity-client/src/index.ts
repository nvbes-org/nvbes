import { createHttpClient, type HttpClient } from '@nvbes/http-client';
import { z } from 'zod';
import type {
  AccessReviewCampaignDetail,
  AccessReviewCampaignExport,
  AccessReviewCampaignsResponse,
  AccessReviewDecisionInput,
  AccessReviewDecisionResponse,
  AccessReviewSchedule,
  AccessReviewSchedulesResponse,
  CloseAccessReviewCampaignInput,
  CreateAccessReviewCampaignInput,
  CreateAccessReviewScheduleInput,
} from './enterprise.access-reviews.schemas';
import {
  activateEnterpriseBreakGlassAccount as activateEnterpriseBreakGlassAccountRequest,
  closeAccessReviewCampaign as closeAccessReviewCampaignRequest,
  createAccessReviewCampaign as createAccessReviewCampaignRequest,
  createAccessReviewSchedule as createAccessReviewScheduleRequest,
  createEnterpriseInvitations as createEnterpriseInvitationsRequest,
  createFederatedIdentityProvider as createFederatedIdentityProviderRequest,
  createTenantDomain as createTenantDomainRequest,
  decideAccessReviewItem as decideAccessReviewItemRequest,
  disableAccessReviewSchedule as disableAccessReviewScheduleRequest,
  enableAccessReviewSchedule as enableAccessReviewScheduleRequest,
  exportAccessReviewCampaign as exportAccessReviewCampaignRequest,
  getAccessReviewCampaign as getAccessReviewCampaignRequest,
  getAccessReviewCampaigns as getAccessReviewCampaignsRequest,
  getAccessReviewSchedules as getAccessReviewSchedulesRequest,
  getEnterpriseAuditEvents as getEnterpriseAuditEventsRequest,
  getEnterpriseBilling as getEnterpriseBillingRequest,
  getEnterpriseContext as getEnterpriseContextRequest,
  getEnterpriseDevelopers as getEnterpriseDevelopersRequest,
  getEnterpriseOverview as getEnterpriseOverviewRequest,
  getEnterprisePolicies as getEnterprisePoliciesRequest,
  getEnterpriseSecurity as getEnterpriseSecurityRequest,
  getEnterpriseTrustCenter as getEnterpriseTrustCenterRequest,
  getEnterpriseUsage as getEnterpriseUsageRequest,
  getEnterpriseUsers as getEnterpriseUsersRequest,
  getEnterpriseWorkspaces as getEnterpriseWorkspacesRequest,
  grantEnterpriseAdminElevation as grantEnterpriseAdminElevationRequest,
  reactivateEnterpriseUser as reactivateEnterpriseUserRequest,
  revokeEnterpriseBreakGlassAccount as revokeEnterpriseBreakGlassAccountRequest,
  revokeEnterpriseDeveloperSecret as revokeEnterpriseDeveloperSecretRequest,
  runAccessReviewScheduleNow as runAccessReviewScheduleNowRequest,
  simulateEnterprisePolicy as simulateEnterprisePolicyRequest,
  suspendEnterpriseUser as suspendEnterpriseUserRequest,
  updateEnterpriseMfaPolicy as updateEnterpriseMfaPolicyRequest,
  updateEnterpriseSessionPolicy as updateEnterpriseSessionPolicyRequest,
  updateEnterpriseUserAccess as updateEnterpriseUserAccessRequest,
  updateTenantDomain as updateTenantDomainRequest,
  verifyTenantDomain as verifyTenantDomainRequest,
} from './enterprise.client';
import type {
  EnterpriseAccessUpdateInput,
  EnterpriseAccessUpdateResponse,
  EnterpriseAdminElevationInput,
  EnterpriseAdminElevationResponse,
  EnterpriseAuditEventsResponse,
  EnterpriseAuditReasonInput,
  EnterpriseBillingResponse,
  EnterpriseBreakGlassInput,
  EnterpriseContextResponse,
  EnterpriseDevelopersResponse,
  EnterpriseInvitationInput,
  EnterpriseInvitationsResponse,
  EnterpriseMfaPolicyInput,
  EnterpriseOverviewResponse,
  EnterprisePoliciesResponse,
  EnterprisePolicySimulationInput,
  EnterprisePolicySimulationResponse,
  EnterpriseReactivateInput,
  EnterpriseSecurityResponse,
  EnterpriseSessionPolicyInput,
  EnterpriseSuspendInput,
  EnterpriseUsageResponse,
  EnterpriseUsersResponse,
  EnterpriseWorkspacesResponse,
} from './enterprise.schemas';
import type { EnterpriseTrustCenterResponse } from './enterprise.trust.schemas';
import type {
  CreateFederatedIdentityProviderInput,
  CreateTenantDomainInput,
  FederatedIdentityProviderResponse,
  TenantDomainResponse,
  UpdateTenantDomainInput,
  VerifyTenantDomainInput,
} from './federation.schemas';

type RequestOptions = { signal?: AbortSignal };

const NullableStringSchema = z.string().nullable();

const AccountWorkspaceSchema = z.object({
  id: z.string(),
  name: z.string(),
  workspace_type: z.string(),
  data_region: z.string().optional(),
  role: z.string(),
  trial_ends_at: NullableStringSchema.optional(),
  plan_code: z.string().optional(),
});

const AccountPrincipalSchema = z.object({
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

const AccountMeSchema = z.object({
  user: AccountPrincipalSchema,
  current_tenant_id: NullableStringSchema,
  current_organization_id: NullableStringSchema,
  current_workspace_id: NullableStringSchema,
  current_workspace_region: NullableStringSchema,
});

const AccountSessionSchema = z.object({
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
  user_agent: NullableStringSchema,
  current: z.boolean(),
});

const SessionsResponseSchema = z.object({
  sessions: z.array(AccountSessionSchema),
});

const AccountEntrySchema = z.object({
  authuser: z.string(),
  status: z.enum(['active', 'expired']).optional(),
  message: NullableStringSchema.optional(),
  user: AccountPrincipalSchema,
  session: AccountSessionSchema,
});

const WorkspacesResponseSchema = z.object({
  workspaces: z.array(AccountWorkspaceSchema).optional(),
});

const AccountsResponseSchema = z.object({
  accounts: z.array(AccountEntrySchema).optional(),
});

const BillingRedirectSchema = z.object({
  url: z.string().url(),
});

const UserConsentSchema = z.object({
  id: z.string(),
  principal_id: z.string(),
  consent_type: z.string(),
  document_version: z.string(),
  ip_address: NullableStringSchema.optional(),
  granted_at: z.string(),
  revoked_at: NullableStringSchema.optional(),
});

const SuccessSchema = z.object({
  success: z.boolean(),
});

const EmptySchema = z.undefined();

const GpcStatusSchema = z.object({
  gpc_enabled: z.boolean(),
  gpc_opt_out_active: z.boolean(),
});

export type AccountWorkspace = z.infer<typeof AccountWorkspaceSchema>;
export type AccountMe = z.infer<typeof AccountMeSchema>;
export type AccountSession = z.infer<typeof AccountSessionSchema>;
export type AccountPrincipal = z.infer<typeof AccountPrincipalSchema>;
export type AccountEntry = z.infer<typeof AccountEntrySchema>;
export type UserConsent = z.infer<typeof UserConsentSchema>;
export type GpcStatus = z.infer<typeof GpcStatusSchema>;

export type IdentityClientOptions = {
  baseUrl?: string;
  http?: HttpClient;
};

export class IdentityClient {
  private readonly http: HttpClient;

  constructor(options: IdentityClientOptions = {}) {
    this.http =
      options.http ??
      createHttpClient({
        baseUrl: options.baseUrl,
        credentials: 'include',
      });
  }

  getMe(options?: { signal?: AbortSignal }): Promise<AccountMe> {
    return this.http.get<AccountMe>('/auth/me', AccountMeSchema, options);
  }

  listWorkspaces(options?: { signal?: AbortSignal }): Promise<AccountWorkspace[]> {
    return this.http
      .get('/workspaces', WorkspacesResponseSchema, options)
      .then((response) => response.workspaces ?? []);
  }

  listAccounts(options?: { signal?: AbortSignal }): Promise<AccountEntry[]> {
    return this.http
      .get('/auth/accounts', AccountsResponseSchema, options)
      .then((response) => response.accounts ?? []);
  }

  listSessions(options?: { signal?: AbortSignal }): Promise<AccountSession[]> {
    return this.http
      .get('/auth/sessions', SessionsResponseSchema, options)
      .then((response) => response.sessions);
  }

  revokeSession(sessionId: string): Promise<void> {
    return this.http.delete(`/auth/sessions/${sessionId}`, SuccessSchema).then(() => undefined);
  }

  forgetAccount(authuser: string): Promise<void> {
    return this.http
      .delete(`/auth/accounts/${encodeURIComponent(authuser)}`, SuccessSchema)
      .then(() => undefined);
  }

  revokeOtherSessions(): Promise<void> {
    return this.http.post('/auth/sessions/revoke-others', SuccessSchema, {}).then(() => undefined);
  }

  logout(): Promise<void> {
    return this.http.post('/auth/logout', SuccessSchema, {}).then(() => undefined);
  }

  createBillingCheckout(workspaceId: string, planCode: string): Promise<string> {
    return this.http
      .post(`/workspaces/${workspaceId}/billing/checkout`, BillingRedirectSchema, {
        plan_code: planCode,
      })
      .then((response) => response.url);
  }

  createBillingPortal(workspaceId: string): Promise<string> {
    return this.http
      .post(`/workspaces/${workspaceId}/billing/portal`, BillingRedirectSchema, {})
      .then((response) => response.url);
  }

  getBillingOverview(workspaceId: string, options?: RequestOptions): Promise<BillingOverview> {
    return this.http.get(
      `/workspaces/${workspaceId}/billing/overview`,
      BillingOverviewSchema,
      options,
    );
  }

  listConsents(options?: { signal?: AbortSignal }): Promise<UserConsent[]> {
    return this.http.get('/legal/consents', z.array(UserConsentSchema), options);
  }

  grantConsent(consentType: string, documentVersion: string): Promise<UserConsent> {
    return this.http.post('/legal/consent', UserConsentSchema, {
      consent_type: consentType,
      document_version: documentVersion,
    });
  }

  gpcStatus(options?: { signal?: AbortSignal }): Promise<GpcStatus> {
    return this.http.get('/legal/gpc', GpcStatusSchema, options);
  }

  revokeConsent(consentType: string, documentVersion: string): Promise<void> {
    return this.http
      .post('/legal/consent/revoke', EmptySchema, {
        consent_type: consentType,
        document_version: documentVersion,
      })
      .then(() => undefined);
  }

  listOAuthClients(options?: RequestOptions): Promise<OAuthClient[]> {
    return this.http
      .get<OAuthClientsResponse>('/oauth/clients', OAuthClientsResponseSchema, options)
      .then((response) => response.clients);
  }

  revokeOAuthClient(clientId: string): Promise<void> {
    return this.http.delete(`/oauth/clients/${clientId}`, SuccessSchema).then(() => undefined);
  }

  createWorkspace(input: CreateWorkspaceInput): Promise<AccountWorkspace> {
    return this.http
      .post('/workspaces', CreateWorkspaceResponseSchema, input)
      .then((response) => response.workspace);
  }

  forgotPassword(email: string): Promise<ForgotPasswordResult> {
    return this.http.post('/auth/password/forgot', ForgotPasswordResultSchema, {
      email,
    });
  }

  resetPassword(token: string, newPassword: string): Promise<ResetPasswordResult> {
    return this.http.post('/auth/password/reset', ResetPasswordResultSchema, {
      token,
      new_password: newPassword,
    });
  }

  getEnterpriseContext(options?: RequestOptions): Promise<EnterpriseContextResponse> {
    return getEnterpriseContextRequest(this.http, options);
  }

  grantEnterpriseAdminElevation(
    input: EnterpriseAdminElevationInput,
    options?: RequestOptions,
  ): Promise<EnterpriseAdminElevationResponse> {
    return grantEnterpriseAdminElevationRequest(this.http, input, options);
  }

  getEnterpriseOverview(options?: RequestOptions): Promise<EnterpriseOverviewResponse> {
    return getEnterpriseOverviewRequest(this.http, options);
  }

  getEnterpriseUsers(options?: RequestOptions): Promise<EnterpriseUsersResponse> {
    return getEnterpriseUsersRequest(this.http, options);
  }

  createEnterpriseInvitations(
    input: EnterpriseInvitationInput,
    options?: RequestOptions,
  ): Promise<EnterpriseInvitationsResponse> {
    return createEnterpriseInvitationsRequest(this.http, input, options);
  }

  updateEnterpriseUserAccess(
    userId: string,
    input: EnterpriseAccessUpdateInput,
    options?: RequestOptions,
  ): Promise<EnterpriseAccessUpdateResponse> {
    return updateEnterpriseUserAccessRequest(this.http, userId, input, options);
  }

  suspendEnterpriseUser(
    userId: string,
    input: EnterpriseSuspendInput,
    options?: RequestOptions,
  ): Promise<EnterpriseAccessUpdateResponse> {
    return suspendEnterpriseUserRequest(this.http, userId, input, options);
  }

  reactivateEnterpriseUser(
    userId: string,
    input: EnterpriseReactivateInput,
    options?: RequestOptions,
  ): Promise<EnterpriseAccessUpdateResponse> {
    return reactivateEnterpriseUserRequest(this.http, userId, input, options);
  }

  activateEnterpriseBreakGlassAccount(
    userId: string,
    input: EnterpriseBreakGlassInput,
    options?: RequestOptions,
  ): Promise<EnterpriseAccessUpdateResponse> {
    return activateEnterpriseBreakGlassAccountRequest(this.http, userId, input, options);
  }

  revokeEnterpriseBreakGlassAccount(
    userId: string,
    input: EnterpriseAuditReasonInput,
    options?: RequestOptions,
  ): Promise<EnterpriseAccessUpdateResponse> {
    return revokeEnterpriseBreakGlassAccountRequest(this.http, userId, input, options);
  }

  getEnterpriseWorkspaces(options?: RequestOptions): Promise<EnterpriseWorkspacesResponse> {
    return getEnterpriseWorkspacesRequest(this.http, options);
  }

  getEnterpriseDevelopers(options?: RequestOptions): Promise<EnterpriseDevelopersResponse> {
    return getEnterpriseDevelopersRequest(this.http, options);
  }

  revokeEnterpriseDeveloperSecret(
    credentialId: string,
    options?: RequestOptions,
  ): Promise<EnterpriseDevelopersResponse> {
    return revokeEnterpriseDeveloperSecretRequest(this.http, credentialId, options);
  }

  getEnterprisePolicies(options?: RequestOptions): Promise<EnterprisePoliciesResponse> {
    return getEnterprisePoliciesRequest(this.http, options);
  }

  updateEnterpriseSessionPolicy(
    input: EnterpriseSessionPolicyInput,
    options?: RequestOptions,
  ): Promise<EnterprisePoliciesResponse> {
    return updateEnterpriseSessionPolicyRequest(this.http, input, options);
  }

  updateEnterpriseMfaPolicy(
    input: EnterpriseMfaPolicyInput,
    options?: RequestOptions,
  ): Promise<EnterprisePoliciesResponse> {
    return updateEnterpriseMfaPolicyRequest(this.http, input, options);
  }

  simulateEnterprisePolicy(
    input: EnterprisePolicySimulationInput,
    options?: RequestOptions,
  ): Promise<EnterprisePolicySimulationResponse> {
    return simulateEnterprisePolicyRequest(this.http, input, options);
  }

  getEnterpriseSecurity(options?: RequestOptions): Promise<EnterpriseSecurityResponse> {
    return getEnterpriseSecurityRequest(this.http, options);
  }

  getEnterpriseAuditEvents(options?: RequestOptions): Promise<EnterpriseAuditEventsResponse> {
    return getEnterpriseAuditEventsRequest(this.http, options);
  }

  getAccessReviewCampaigns(options?: RequestOptions): Promise<AccessReviewCampaignsResponse> {
    return getAccessReviewCampaignsRequest(this.http, options);
  }

  getAccessReviewSchedules(options?: RequestOptions): Promise<AccessReviewSchedulesResponse> {
    return getAccessReviewSchedulesRequest(this.http, options);
  }

  getAccessReviewCampaign(
    campaignId: string,
    options?: RequestOptions,
  ): Promise<AccessReviewCampaignDetail> {
    return getAccessReviewCampaignRequest(this.http, campaignId, options);
  }

  exportAccessReviewCampaign(
    campaignId: string,
    options?: RequestOptions,
  ): Promise<AccessReviewCampaignExport> {
    return exportAccessReviewCampaignRequest(this.http, campaignId, options);
  }

  closeAccessReviewCampaign(
    campaignId: string,
    input: CloseAccessReviewCampaignInput,
    options?: RequestOptions,
  ): Promise<AccessReviewCampaignDetail> {
    return closeAccessReviewCampaignRequest(this.http, campaignId, input, options);
  }

  createAccessReviewCampaign(
    input: CreateAccessReviewCampaignInput,
    options?: RequestOptions,
  ): Promise<AccessReviewCampaignDetail> {
    return createAccessReviewCampaignRequest(this.http, input, options);
  }

  createAccessReviewSchedule(
    input: CreateAccessReviewScheduleInput,
    options?: RequestOptions,
  ): Promise<AccessReviewSchedule> {
    return createAccessReviewScheduleRequest(this.http, input, options);
  }

  disableAccessReviewSchedule(
    scheduleId: string,
    options?: RequestOptions,
  ): Promise<AccessReviewSchedule> {
    return disableAccessReviewScheduleRequest(this.http, scheduleId, options);
  }

  enableAccessReviewSchedule(
    scheduleId: string,
    options?: RequestOptions,
  ): Promise<AccessReviewSchedule> {
    return enableAccessReviewScheduleRequest(this.http, scheduleId, options);
  }

  runAccessReviewScheduleNow(
    scheduleId: string,
    options?: RequestOptions,
  ): Promise<AccessReviewCampaignDetail> {
    return runAccessReviewScheduleNowRequest(this.http, scheduleId, options);
  }

  decideAccessReviewItem(
    campaignId: string,
    itemId: string,
    input: AccessReviewDecisionInput,
    options?: RequestOptions,
  ): Promise<AccessReviewDecisionResponse> {
    return decideAccessReviewItemRequest(this.http, campaignId, itemId, input, options);
  }

  getEnterpriseTrustCenter(options?: RequestOptions): Promise<EnterpriseTrustCenterResponse> {
    return getEnterpriseTrustCenterRequest(this.http, options);
  }

  createFederatedIdentityProvider(
    tenantId: string,
    input: CreateFederatedIdentityProviderInput,
    options?: RequestOptions,
  ): Promise<FederatedIdentityProviderResponse> {
    return createFederatedIdentityProviderRequest(this.http, tenantId, input, options);
  }

  createTenantDomain(
    tenantId: string,
    input: CreateTenantDomainInput,
    options?: RequestOptions,
  ): Promise<TenantDomainResponse> {
    return createTenantDomainRequest(this.http, tenantId, input, options);
  }

  updateTenantDomain(
    tenantId: string,
    domainId: string,
    input: UpdateTenantDomainInput,
    options?: RequestOptions,
  ): Promise<TenantDomainResponse> {
    return updateTenantDomainRequest(this.http, tenantId, domainId, input, options);
  }

  verifyTenantDomain(
    tenantId: string,
    domainId: string,
    input: VerifyTenantDomainInput,
    options?: RequestOptions,
  ): Promise<TenantDomainResponse> {
    return verifyTenantDomainRequest(this.http, tenantId, domainId, input, options);
  }

  getEnterpriseBilling(options?: RequestOptions): Promise<EnterpriseBillingResponse> {
    return getEnterpriseBillingRequest(this.http, options);
  }

  getEnterpriseUsage(options?: RequestOptions): Promise<EnterpriseUsageResponse> {
    return getEnterpriseUsageRequest(this.http, options);
  }
}

const OAuthClientSchema = z.object({
  id: z.string(),
  client_id: z.string(),
  name: z.string(),
  redirect_uris: z.array(z.string()),
  created_at: z.string(),
  tenant_id: z.string().nullable().optional(),
  owner_scope_type: z.string(),
  owner_scope_id: z.string(),
  client_type: z.string(),
});

const OAuthClientsResponseSchema = z.object({
  clients: z.array(OAuthClientSchema),
});

const CreateWorkspaceInputSchema = z.object({
  name: z.string().min(1).max(100),
  workspace_type: z.string().optional(),
});

const CreateWorkspaceResponseSchema = z.object({
  workspace: AccountWorkspaceSchema,
});

const ForgotPasswordResultSchema = z.object({
  success: z.boolean(),
  requires_admin_approval: z.boolean(),
  available_at: NullableStringSchema.optional(),
});

const ResetPasswordResultSchema = z.object({
  success: z.boolean(),
});

const PlanViewSchema = z.object({
  code: z.string(),
  included_storage_gb: z.number(),
  included_users: z.number(),
  retention_days: z.number(),
  max_share_links: z.number(),
  audit_level: z.string(),
  max_share_link_ttl_days: z.number(),
  monthly_price_cents: z.number(),
  currency: z.string(),
});

const SubscriptionViewSchema = z.object({
  status: z.string(),
  billing_provider: z.string(),
  billing_customer_id: z.string().nullable().optional(),
  billing_subscription_id: z.string().nullable().optional(),
  current_period_start: z.string().nullable().optional(),
  current_period_end: z.string().nullable().optional(),
  trial_ends_at: z.string().nullable().optional(),
});

const BillingOverviewSchema = z.object({
  workspace_id: z.string(),
  plan: PlanViewSchema,
  subscription: SubscriptionViewSchema,
  billing_account: z.object({
    stripe_customer_id: z.string().nullable().optional(),
    billing_email: z.string().nullable().optional(),
    country: z.string().nullable().optional(),
    customer_type: z.string(),
    vat_number: z.string().nullable().optional(),
    tax_exempt_status: z.string().nullable().optional(),
  }),
  entitlements: z.object({
    can_upload: z.boolean(),
    can_create_share_links: z.boolean(),
    included_storage_bytes: z.number(),
    included_users: z.number(),
    max_share_links: z.number(),
    max_share_link_ttl_days: z.number(),
    audit_level: z.string(),
    api_key_limit: z.number(),
    billing_locked: z.boolean(),
  }),
  invoice_estimate: z.object({
    workspace_id: z.string(),
    billing_period_start: z.string(),
    billing_period_end: z.string(),
    base_amount_cents: z.number(),
    storage_overage_amount_cents: z.number(),
    seat_overage_amount_cents: z.number(),
    estimated_amount_cents: z.number(),
    currency: z.string(),
  }),
});

export type OAuthClient = z.infer<typeof OAuthClientSchema>;
export type OAuthClientsResponse = z.infer<typeof OAuthClientsResponseSchema>;
export type CreateWorkspaceInput = z.infer<typeof CreateWorkspaceInputSchema>;
export type ForgotPasswordResult = z.infer<typeof ForgotPasswordResultSchema>;
export type ResetPasswordResult = z.infer<typeof ResetPasswordResultSchema>;
export type BillingOverview = z.infer<typeof BillingOverviewSchema>;

export const identityClient = new IdentityClient();

export function createIdentityClient(options?: IdentityClientOptions): IdentityClient {
  return new IdentityClient(options);
}

export * from './enterprise.access-reviews.schemas';
export * from './enterprise.client';
export * from './enterprise.schemas';
export * from './enterprise.trust.schemas';
export * from './federation.schemas';
