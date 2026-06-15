import type { HttpClient, HttpRequestOptions } from "@nvbes/http-client";
import {
	type AccessReviewCampaignDetail,
	AccessReviewCampaignDetailSchema,
	type AccessReviewCampaignsResponse,
	AccessReviewCampaignsResponseSchema,
	type CreateAccessReviewCampaignInput,
	CreateAccessReviewCampaignInputSchema,
	type EnterpriseAccessUpdateInput,
	type EnterpriseAccessUpdateResponse,
	EnterpriseAccessUpdateInputSchema,
	EnterpriseAccessUpdateResponseSchema,
	type EnterpriseAuditEventsResponse,
	EnterpriseAuditEventsResponseSchema,
	type EnterpriseBillingResponse,
	EnterpriseBillingResponseSchema,
	type EnterpriseContextResponse,
	EnterpriseContextResponseSchema,
	type EnterpriseDevelopersResponse,
	EnterpriseDevelopersResponseSchema,
	type EnterpriseInvitationInput,
	EnterpriseInvitationInputSchema,
	type EnterpriseInvitationsResponse,
	EnterpriseInvitationsResponseSchema,
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
	type EnterpriseSuspendInput,
	EnterpriseSuspendInputSchema,
	type EnterpriseUsageResponse,
	EnterpriseUsageResponseSchema,
	type EnterpriseUsersResponse,
	EnterpriseUsersResponseSchema,
	type EnterpriseWorkspacesResponse,
	EnterpriseWorkspacesResponseSchema,
} from "./enterprise.schemas";
import {
	type EnterpriseTrustCenterResponse,
	EnterpriseTrustCenterResponseSchema,
} from "./enterprise.trust.schemas";

export type EnterpriseRequestOptions = Pick<HttpRequestOptions, "signal">;

const EnterpriseApiBasePath = "/api/v1/enterprise";

export function getEnterpriseContext(
	http: HttpClient,
	options?: EnterpriseRequestOptions,
): Promise<EnterpriseContextResponse> {
	return http.get(
		`${EnterpriseApiBasePath}/context`,
		EnterpriseContextResponseSchema,
		options,
	);
}

export function getEnterpriseOverview(
	http: HttpClient,
	options?: EnterpriseRequestOptions,
): Promise<EnterpriseOverviewResponse> {
	return http.get(
		`${EnterpriseApiBasePath}/overview`,
		EnterpriseOverviewResponseSchema,
		options,
	);
}

export function getEnterpriseUsers(
	http: HttpClient,
	options?: EnterpriseRequestOptions,
): Promise<EnterpriseUsersResponse> {
	return http.get(
		`${EnterpriseApiBasePath}/users`,
		EnterpriseUsersResponseSchema,
		options,
	);
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
			method: "PATCH",
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

export function getEnterprisePolicies(
	http: HttpClient,
	options?: EnterpriseRequestOptions,
): Promise<EnterprisePoliciesResponse> {
	return http.get(
		`${EnterpriseApiBasePath}/policies`,
		EnterprisePoliciesResponseSchema,
		options,
	);
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
	return http.get(
		`${EnterpriseApiBasePath}/security`,
		EnterpriseSecurityResponseSchema,
		options,
	);
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

export function getEnterpriseBilling(
	http: HttpClient,
	options?: EnterpriseRequestOptions,
): Promise<EnterpriseBillingResponse> {
	return http.get(
		`${EnterpriseApiBasePath}/billing`,
		EnterpriseBillingResponseSchema,
		options,
	);
}

export function getEnterpriseUsage(
	http: HttpClient,
	options?: EnterpriseRequestOptions,
): Promise<EnterpriseUsageResponse> {
	return http.get(
		`${EnterpriseApiBasePath}/usage`,
		EnterpriseUsageResponseSchema,
		options,
	);
}
