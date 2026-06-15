import type { HttpClient, HttpRequestOptions } from '@nvbes/http-client';
import {
  type AccessReviewCampaignDetail,
  AccessReviewCampaignDetailSchema,
  type AccessReviewCampaignExport,
  AccessReviewCampaignExportSchema,
  type AccessReviewCampaignsResponse,
  AccessReviewCampaignsResponseSchema,
  type AccessReviewDecisionInput,
  AccessReviewDecisionInputSchema,
  type AccessReviewDecisionResponse,
  AccessReviewDecisionResponseSchema,
  type AccessReviewSchedule,
  AccessReviewScheduleSchema,
  type AccessReviewSchedulesResponse,
  AccessReviewSchedulesResponseSchema,
  type CloseAccessReviewCampaignInput,
  CloseAccessReviewCampaignInputSchema,
  type CreateAccessReviewCampaignInput,
  CreateAccessReviewCampaignInputSchema,
  type CreateAccessReviewScheduleInput,
  CreateAccessReviewScheduleInputSchema,
} from './enterprise.access-reviews.schemas';
import {
  type EnterpriseAccessUpdateInput,
  EnterpriseAccessUpdateInputSchema,
  type EnterpriseAccessUpdateResponse,
  EnterpriseAccessUpdateResponseSchema,
  type EnterpriseAdminElevationInput,
  EnterpriseAdminElevationInputSchema,
  type EnterpriseAdminElevationResponse,
  EnterpriseAdminElevationResponseSchema,
  type EnterpriseAuditEventsResponse,
  EnterpriseAuditEventsResponseSchema,
  type EnterpriseAuditReasonInput,
  EnterpriseAuditReasonInputSchema,
  type EnterpriseBillingResponse,
  EnterpriseBillingResponseSchema,
  type EnterpriseBreakGlassInput,
  EnterpriseBreakGlassInputSchema,
  type EnterpriseContextResponse,
  EnterpriseContextResponseSchema,
  type EnterpriseDevelopersResponse,
  EnterpriseDevelopersResponseSchema,
  type EnterpriseInvitationInput,
  EnterpriseInvitationInputSchema,
  type EnterpriseInvitationsResponse,
  EnterpriseInvitationsResponseSchema,
  type EnterpriseMfaPolicyInput,
  EnterpriseMfaPolicyInputSchema,
  type EnterpriseOverviewResponse,
  EnterpriseOverviewResponseSchema,
  type EnterprisePoliciesResponse,
  EnterprisePoliciesResponseSchema,
  type EnterprisePolicySimulationInput,
  EnterprisePolicySimulationInputSchema,
  type EnterprisePolicySimulationResponse,
  EnterprisePolicySimulationResponseSchema,
  type EnterpriseReactivateInput,
  EnterpriseReactivateInputSchema,
  type EnterpriseSecurityResponse,
  EnterpriseSecurityResponseSchema,
  type EnterpriseSessionPolicyInput,
  EnterpriseSessionPolicyInputSchema,
  type EnterpriseSuspendInput,
  EnterpriseSuspendInputSchema,
  type EnterpriseUsageResponse,
  EnterpriseUsageResponseSchema,
  type EnterpriseUsersResponse,
  EnterpriseUsersResponseSchema,
  type EnterpriseWorkspacesResponse,
  EnterpriseWorkspacesResponseSchema,
} from './enterprise.schemas';
import {
  type EnterpriseTrustCenterResponse,
  EnterpriseTrustCenterResponseSchema,
} from './enterprise.trust.schemas';
import {
  type CreateFederatedIdentityProviderInput,
  CreateFederatedIdentityProviderInputSchema,
  type CreateTenantDomainInput,
  CreateTenantDomainInputSchema,
  type FederatedIdentityProviderResponse,
  FederatedIdentityProviderResponseSchema,
  type TenantDomainResponse,
  TenantDomainResponseSchema,
  type UpdateTenantDomainInput,
  UpdateTenantDomainInputSchema,
  type VerifyTenantDomainInput,
  VerifyTenantDomainInputSchema,
} from './federation.schemas';

export type EnterpriseRequestOptions = Pick<HttpRequestOptions, 'signal'>;

const EnterpriseApiBasePath = '/api/v1/enterprise';

export function getEnterpriseContext(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseContextResponse> {
  return http.get(`${EnterpriseApiBasePath}/context`, EnterpriseContextResponseSchema, options);
}

export function grantEnterpriseAdminElevation(
  http: HttpClient,
  input: EnterpriseAdminElevationInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAdminElevationResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/admin-elevation`,
    EnterpriseAdminElevationResponseSchema,
    EnterpriseAdminElevationInputSchema.parse(input),
    options,
  );
}

export function getEnterpriseOverview(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseOverviewResponse> {
  return http.get(`${EnterpriseApiBasePath}/overview`, EnterpriseOverviewResponseSchema, options);
}

export function getEnterpriseUsers(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseUsersResponse> {
  return http.get(`${EnterpriseApiBasePath}/users`, EnterpriseUsersResponseSchema, options);
}

export function createEnterpriseInvitations(
  http: HttpClient,
  input: EnterpriseInvitationInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseInvitationsResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/invitations`,
    EnterpriseInvitationsResponseSchema,
    EnterpriseInvitationInputSchema.parse(input),
    options,
  );
}

export function updateEnterpriseUserAccess(
  http: HttpClient,
  userId: string,
  input: EnterpriseAccessUpdateInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.request(
    `${EnterpriseApiBasePath}/users/${encodeURIComponent(userId)}/access`,
    EnterpriseAccessUpdateResponseSchema,
    {
      ...options,
      body: EnterpriseAccessUpdateInputSchema.parse(input),
      method: 'PATCH',
    },
  );
}

export function suspendEnterpriseUser(
  http: HttpClient,
  userId: string,
  input: EnterpriseSuspendInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/users/${encodeURIComponent(userId)}/suspend`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseSuspendInputSchema.parse(input),
    options,
  );
}

export function reactivateEnterpriseUser(
  http: HttpClient,
  userId: string,
  input: EnterpriseReactivateInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/users/${encodeURIComponent(userId)}/reactivate`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseReactivateInputSchema.parse(input),
    options,
  );
}

export function activateEnterpriseBreakGlassAccount(
  http: HttpClient,
  userId: string,
  input: EnterpriseBreakGlassInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/users/${encodeURIComponent(userId)}/break-glass`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseBreakGlassInputSchema.parse(input),
    options,
  );
}

export function revokeEnterpriseBreakGlassAccount(
  http: HttpClient,
  userId: string,
  input: EnterpriseAuditReasonInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/users/${encodeURIComponent(userId)}/break-glass/revoke`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseAuditReasonInputSchema.parse(input),
    options,
  );
}

export function getEnterpriseWorkspaces(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseWorkspacesResponse> {
  return http.get(
    `${EnterpriseApiBasePath}/workspaces`,
    EnterpriseWorkspacesResponseSchema,
    options,
  );
}

export function getEnterpriseDevelopers(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseDevelopersResponse> {
  return http.get(
    `${EnterpriseApiBasePath}/developers`,
    EnterpriseDevelopersResponseSchema,
    options,
  );
}

export function revokeEnterpriseDeveloperSecret(
  http: HttpClient,
  credentialId: string,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseDevelopersResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/developers/credentials/${encodeURIComponent(credentialId)}/revoke`,
    EnterpriseDevelopersResponseSchema,
    undefined,
    options,
  );
}

export function getEnterprisePolicies(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterprisePoliciesResponse> {
  return http.get(`${EnterpriseApiBasePath}/policies`, EnterprisePoliciesResponseSchema, options);
}

export function updateEnterpriseSessionPolicy(
  http: HttpClient,
  input: EnterpriseSessionPolicyInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterprisePoliciesResponse> {
  return http.request(
    `${EnterpriseApiBasePath}/policies/session`,
    EnterprisePoliciesResponseSchema,
    {
      ...options,
      body: EnterpriseSessionPolicyInputSchema.parse(input),
      method: 'PATCH',
    },
  );
}

export function updateEnterpriseMfaPolicy(
  http: HttpClient,
  input: EnterpriseMfaPolicyInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterprisePoliciesResponse> {
  return http.request(`${EnterpriseApiBasePath}/policies/mfa`, EnterprisePoliciesResponseSchema, {
    ...options,
    body: EnterpriseMfaPolicyInputSchema.parse(input),
    method: 'PATCH',
  });
}

export function simulateEnterprisePolicy(
  http: HttpClient,
  input: EnterprisePolicySimulationInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterprisePolicySimulationResponse> {
  return http.post(
    `${EnterpriseApiBasePath}/policies/simulate`,
    EnterprisePolicySimulationResponseSchema,
    EnterprisePolicySimulationInputSchema.parse(input),
    options,
  );
}

export function getEnterpriseSecurity(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseSecurityResponse> {
  return http.get(`${EnterpriseApiBasePath}/security`, EnterpriseSecurityResponseSchema, options);
}

export function getEnterpriseAuditEvents(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAuditEventsResponse> {
  return http.get(
    `${EnterpriseApiBasePath}/audit-events`,
    EnterpriseAuditEventsResponseSchema,
    options,
  );
}

export function getAccessReviewCampaigns(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewCampaignsResponse> {
  return http.get(
    `${EnterpriseApiBasePath}/access-review-campaigns`,
    AccessReviewCampaignsResponseSchema,
    options,
  );
}

export function getAccessReviewSchedules(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewSchedulesResponse> {
  return http.get(
    `${EnterpriseApiBasePath}/access-review-schedules`,
    AccessReviewSchedulesResponseSchema,
    options,
  );
}

export function getAccessReviewCampaign(
  http: HttpClient,
  campaignId: string,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewCampaignDetail> {
  return http.get(
    `${EnterpriseApiBasePath}/access-review-campaigns/${encodeURIComponent(campaignId)}`,
    AccessReviewCampaignDetailSchema,
    options,
  );
}

export function exportAccessReviewCampaign(
  http: HttpClient,
  campaignId: string,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewCampaignExport> {
  return http.get(
    `${EnterpriseApiBasePath}/access-review-campaigns/${encodeURIComponent(campaignId)}/export`,
    AccessReviewCampaignExportSchema,
    options,
  );
}

export function closeAccessReviewCampaign(
  http: HttpClient,
  campaignId: string,
  input: CloseAccessReviewCampaignInput,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewCampaignDetail> {
  return http.post(
    `${EnterpriseApiBasePath}/access-review-campaigns/${encodeURIComponent(campaignId)}/close`,
    AccessReviewCampaignDetailSchema,
    CloseAccessReviewCampaignInputSchema.parse(input),
    options,
  );
}

export function createAccessReviewSchedule(
  http: HttpClient,
  input: CreateAccessReviewScheduleInput,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewSchedule> {
  return http.post(
    `${EnterpriseApiBasePath}/access-review-schedules`,
    AccessReviewScheduleSchema,
    CreateAccessReviewScheduleInputSchema.parse(input),
    options,
  );
}

export function disableAccessReviewSchedule(
  http: HttpClient,
  scheduleId: string,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewSchedule> {
  return http.post(
    `${EnterpriseApiBasePath}/access-review-schedules/${encodeURIComponent(scheduleId)}/disable`,
    AccessReviewScheduleSchema,
    undefined,
    options,
  );
}

export function enableAccessReviewSchedule(
  http: HttpClient,
  scheduleId: string,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewSchedule> {
  return http.post(
    `${EnterpriseApiBasePath}/access-review-schedules/${encodeURIComponent(scheduleId)}/enable`,
    AccessReviewScheduleSchema,
    undefined,
    options,
  );
}

export function runAccessReviewScheduleNow(
  http: HttpClient,
  scheduleId: string,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewCampaignDetail> {
  return http.post(
    `${EnterpriseApiBasePath}/access-review-schedules/${encodeURIComponent(scheduleId)}/run`,
    AccessReviewCampaignDetailSchema,
    undefined,
    options,
  );
}

export function createAccessReviewCampaign(
  http: HttpClient,
  input: CreateAccessReviewCampaignInput,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewCampaignDetail> {
  return http.post(
    `${EnterpriseApiBasePath}/access-review-campaigns`,
    AccessReviewCampaignDetailSchema,
    CreateAccessReviewCampaignInputSchema.parse(input),
    options,
  );
}

export function decideAccessReviewItem(
  http: HttpClient,
  campaignId: string,
  itemId: string,
  input: AccessReviewDecisionInput,
  options?: EnterpriseRequestOptions,
): Promise<AccessReviewDecisionResponse> {
  return http.request(
    `${EnterpriseApiBasePath}/access-review-campaigns/${encodeURIComponent(campaignId)}/items/${encodeURIComponent(itemId)}`,
    AccessReviewDecisionResponseSchema,
    {
      ...options,
      body: AccessReviewDecisionInputSchema.parse(input),
      method: 'PATCH',
    },
  );
}

export function getEnterpriseTrustCenter(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseTrustCenterResponse> {
  return http.get(
    `${EnterpriseApiBasePath}/trust-center`,
    EnterpriseTrustCenterResponseSchema,
    options,
  );
}

export function createTenantDomain(
  http: HttpClient,
  tenantId: string,
  input: CreateTenantDomainInput,
  options?: EnterpriseRequestOptions,
): Promise<TenantDomainResponse> {
  return http.post(
    `/tenants/${encodeURIComponent(tenantId)}/domains`,
    TenantDomainResponseSchema,
    CreateTenantDomainInputSchema.parse(input),
    options,
  );
}

export function createFederatedIdentityProvider(
  http: HttpClient,
  tenantId: string,
  input: CreateFederatedIdentityProviderInput,
  options?: EnterpriseRequestOptions,
): Promise<FederatedIdentityProviderResponse> {
  return http.post(
    `/tenants/${encodeURIComponent(tenantId)}/identity-providers`,
    FederatedIdentityProviderResponseSchema,
    CreateFederatedIdentityProviderInputSchema.parse(input),
    options,
  );
}

export function updateTenantDomain(
  http: HttpClient,
  tenantId: string,
  domainId: string,
  input: UpdateTenantDomainInput,
  options?: EnterpriseRequestOptions,
): Promise<TenantDomainResponse> {
  return http.request(
    `/tenants/${encodeURIComponent(tenantId)}/domains/${encodeURIComponent(domainId)}`,
    TenantDomainResponseSchema,
    {
      ...options,
      body: UpdateTenantDomainInputSchema.parse(input),
      method: 'PATCH',
    },
  );
}

export function verifyTenantDomain(
  http: HttpClient,
  tenantId: string,
  domainId: string,
  input: VerifyTenantDomainInput,
  options?: EnterpriseRequestOptions,
): Promise<TenantDomainResponse> {
  return http.post(
    `/tenants/${encodeURIComponent(tenantId)}/domains/${encodeURIComponent(domainId)}/verify`,
    TenantDomainResponseSchema,
    VerifyTenantDomainInputSchema.parse(input),
    options,
  );
}

export function getEnterpriseBilling(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseBillingResponse> {
  return http.get(`${EnterpriseApiBasePath}/billing`, EnterpriseBillingResponseSchema, options);
}

export function getEnterpriseUsage(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseUsageResponse> {
  return http.get(`${EnterpriseApiBasePath}/usage`, EnterpriseUsageResponseSchema, options);
}
