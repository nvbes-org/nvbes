import { queryOptions } from '@tanstack/react-query';
import { enterpriseClient } from './enterprise.api';

export const enterpriseQueryKeys = {
  accessReviewCampaigns: ['enterprise', 'access-review-campaigns'] as const,
  accessReviewSchedules: ['enterprise', 'access-review-schedules'] as const,
  context: ['enterprise', 'context'] as const,
  developers: ['enterprise', 'developers'] as const,
  policies: ['enterprise', 'policies'] as const,
  security: ['enterprise', 'security'] as const,
  trustCenter: ['enterprise', 'trust-center'] as const,
  users: ['enterprise', 'users'] as const,
  workspaces: ['enterprise', 'workspaces'] as const,
};

export function enterpriseContextQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.context,
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseContext({ signal }),
  });
}

export function accessReviewSchedulesQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.accessReviewSchedules,
    queryFn: ({ signal }) => enterpriseClient.getAccessReviewSchedules({ signal }),
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
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseWorkspaces({ signal }),
  });
}

export function enterpriseDevelopersQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.developers,
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseDevelopers({ signal }),
  });
}

export function enterprisePoliciesQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.policies,
    queryFn: ({ signal }) => enterpriseClient.getEnterprisePolicies({ signal }),
  });
}

export function enterpriseSecurityQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.security,
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseSecurity({ signal }),
  });
}

export function enterpriseTrustCenterQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.trustCenter,
    queryFn: ({ signal }) => enterpriseClient.getEnterpriseTrustCenter({ signal }),
  });
}

export function accessReviewCampaignsQueryOptions() {
  return queryOptions({
    queryKey: enterpriseQueryKeys.accessReviewCampaigns,
    queryFn: ({ signal }) => enterpriseClient.getAccessReviewCampaigns({ signal }),
  });
}
