import { queryOptions } from "@tanstack/react-query";
import { enterpriseClient } from "./enterprise.api";

export const enterpriseQueryKeys = {
	accessReviewCampaigns: ["enterprise", "access-review-campaigns"] as const,
	context: ["enterprise", "context"] as const,
	policies: ["enterprise", "policies"] as const,
	users: ["enterprise", "users"] as const,
	workspaces: ["enterprise", "workspaces"] as const,
};

export function enterpriseContextQueryOptions() {
	return queryOptions({
		queryKey: enterpriseQueryKeys.context,
		queryFn: ({ signal }) => enterpriseClient.getEnterpriseContext({ signal }),
	});
}

export function enterpriseUsersQueryOptions() {
	return queryOptions({
		queryKey: enterpriseQueryKeys.users,
		queryFn: ({ signal }) => enterpriseClient.getEnterpriseUsers({ signal }),
	});
}

export function enterpriseWorkspacesQueryOptions() {
	return queryOptions({
		queryKey: enterpriseQueryKeys.workspaces,
		queryFn: ({ signal }) =>
			enterpriseClient.getEnterpriseWorkspaces({ signal }),
	});
}

export function enterprisePoliciesQueryOptions() {
	return queryOptions({
		queryKey: enterpriseQueryKeys.policies,
		queryFn: ({ signal }) => enterpriseClient.getEnterprisePolicies({ signal }),
	});
}

export function accessReviewCampaignsQueryOptions() {
	return queryOptions({
		queryKey: enterpriseQueryKeys.accessReviewCampaigns,
		queryFn: ({ signal }) =>
			enterpriseClient.getAccessReviewCampaigns({ signal }),
	});
}
