import type { HttpClient, HttpRequestOptions } from '@nvbes/http-client';
import {
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
} from './enterprise.schemas';

export type EnterpriseRequestOptions = Pick<HttpRequestOptions, 'signal'>;

export function getEnterpriseContext(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseContextResponse> {
  return http.get('/enterprise/context', EnterpriseContextResponseSchema, options);
}

export function getEnterpriseOverview(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseOverviewResponse> {
  return http.get('/enterprise/overview', EnterpriseOverviewResponseSchema, options);
}

export function getEnterpriseUsers(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseUsersResponse> {
  return http.get('/enterprise/users', EnterpriseUsersResponseSchema, options);
}

export function createEnterpriseInvitations(
  http: HttpClient,
  input: EnterpriseInvitationInput,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseInvitationsResponse> {
  return http.post(
    '/enterprise/invitations',
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
  return http.post(
    `/enterprise/users/${encodeURIComponent(userId)}/access`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseAccessUpdateInputSchema.parse(input),
    options,
  );
}

export function suspendEnterpriseUser(
  http: HttpClient,
  userId: string,
  input: EnterpriseSuspendInput = {},
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.post(
    `/enterprise/users/${encodeURIComponent(userId)}/suspend`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseSuspendInputSchema.parse(input),
    options,
  );
}

export function reactivateEnterpriseUser(
  http: HttpClient,
  userId: string,
  input: EnterpriseReactivateInput = {},
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAccessUpdateResponse> {
  return http.post(
    `/enterprise/users/${encodeURIComponent(userId)}/reactivate`,
    EnterpriseAccessUpdateResponseSchema,
    EnterpriseReactivateInputSchema.parse(input),
    options,
  );
}

export function getEnterpriseWorkspaces(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseWorkspacesResponse> {
  return http.get('/enterprise/workspaces', EnterpriseWorkspacesResponseSchema, options);
}

export function getEnterpriseDevelopers(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseDevelopersResponse> {
  return http.get('/enterprise/developers', EnterpriseDevelopersResponseSchema, options);
}

export function getEnterprisePolicies(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterprisePoliciesResponse> {
  return http.get('/enterprise/policies', EnterprisePoliciesResponseSchema, options);
}

export function getEnterpriseSecurity(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseSecurityResponse> {
  return http.get('/enterprise/security', EnterpriseSecurityResponseSchema, options);
}

export function getEnterpriseAuditEvents(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseAuditEventsResponse> {
  return http.get('/enterprise/audit-events', EnterpriseAuditEventsResponseSchema, options);
}

export function getEnterpriseBilling(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseBillingResponse> {
  return http.get('/enterprise/billing', EnterpriseBillingResponseSchema, options);
}

export function getEnterpriseUsage(
  http: HttpClient,
  options?: EnterpriseRequestOptions,
): Promise<EnterpriseUsageResponse> {
  return http.get('/enterprise/usage', EnterpriseUsageResponseSchema, options);
}
