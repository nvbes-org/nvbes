import {
  AccountIdentityClient,
  type IdentityClientOptions,
  type RequestOptions,
} from './identity.account-client';
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

export class IdentityClient extends AccountIdentityClient {
  constructor(options: IdentityClientOptions = {}) {
    super(options);
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

  getEnterpriseUsage(options?: RequestOptions): Promise<EnterpriseUsageResponse> {
    return getEnterpriseUsageRequest(this.http, options);
  }
}
