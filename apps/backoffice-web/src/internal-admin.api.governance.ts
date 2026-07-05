import { verifiedFetch } from '@nvbes/web-runtime';
import type {
  AccessCenterSnapshot,
  AdminCredentials,
  IdentityGovernanceSnapshot,
  SecurityCenterSnapshot,
  TenantDetail,
  UserDetail,
  WorkspaceDetail,
} from './internal-admin.types';
import type { OperationBody, OperationOk, OperationPath } from './internal-admin.generated-api';
import { authHeaders, parseJson, postJson } from './internal-admin.api.core';

export async function getAccessCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/access-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<AccessCenterSnapshot>(response);
}

export function suspendWorkspaceMembership(
  credentials: AdminCredentials,
  workspaceId: OperationPath<'suspendWorkspaceMembership'>['workspaceId'],
  principalId: OperationPath<'suspendWorkspaceMembership'>['principalId'],
  body: OperationBody<'suspendWorkspaceMembership'>,
) {
  return postJson<
    OperationBody<'suspendWorkspaceMembership'>,
    OperationOk<'suspendWorkspaceMembership'>
  >(
    credentials,
    `/admin/access-center/workspace-memberships/${workspaceId}/${principalId}/suspend`,
    body,
  );
}

export async function getSecurityCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/security-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<SecurityCenterSnapshot>(response);
}

export function revokeMfaFactor(
  credentials: AdminCredentials,
  factorId: OperationPath<'revokeMfaFactor'>['factorId'],
  body: OperationBody<'revokeMfaFactor'>,
) {
  return postJson<OperationBody<'revokeMfaFactor'>, OperationOk<'revokeMfaFactor'>>(
    credentials,
    `/admin/security-center/mfa-factors/${factorId}/revoke`,
    body,
  );
}

export function revokeOauthConsent(
  credentials: AdminCredentials,
  consentId: OperationPath<'revokeOauthConsent'>['consentId'],
  body: OperationBody<'revokeOauthConsent'>,
) {
  return postJson<OperationBody<'revokeOauthConsent'>, OperationOk<'revokeOauthConsent'>>(
    credentials,
    `/admin/security-center/oauth-consents/${consentId}/revoke`,
    body,
  );
}

export async function getIdentityGovernanceCenter(credentials: AdminCredentials) {
  const response = await verifiedFetch('/admin/identity-governance-center', {
    headers: authHeaders(credentials),
  });
  return parseJson<IdentityGovernanceSnapshot>(response);
}

export function revokeBreakGlassAccount(
  credentials: AdminCredentials,
  tenantId: OperationPath<'revokeBreakGlassAccount'>['tenantId'],
  principalId: OperationPath<'revokeBreakGlassAccount'>['principalId'],
  body: OperationBody<'revokeBreakGlassAccount'>,
) {
  return postJson<OperationBody<'revokeBreakGlassAccount'>, OperationOk<'revokeBreakGlassAccount'>>(
    credentials,
    `/admin/identity-governance-center/break-glass/${tenantId}/${principalId}/revoke`,
    body,
  );
}

export function cancelRecoveryRequest(
  credentials: AdminCredentials,
  requestId: OperationPath<'cancelRecoveryRequest'>['requestId'],
  body: OperationBody<'cancelRecoveryRequest'>,
) {
  return postJson<OperationBody<'cancelRecoveryRequest'>, OperationOk<'cancelRecoveryRequest'>>(
    credentials,
    `/admin/identity-governance-center/recovery-requests/${requestId}/cancel`,
    body,
  );
}

export function grantOperatorRole(
  credentials: AdminCredentials,
  principalId: OperationPath<'grantOperatorRole'>['principalId'],
  role: OperationPath<'grantOperatorRole'>['role'],
  body: OperationBody<'grantOperatorRole'>,
) {
  return postJson<OperationBody<'grantOperatorRole'>, OperationOk<'grantOperatorRole'>>(
    credentials,
    `/admin/identity-governance-center/operator-grants/${principalId}/${role}/grant`,
    body,
  );
}

export function revokeOperatorRole(
  credentials: AdminCredentials,
  principalId: OperationPath<'revokeOperatorRole'>['principalId'],
  role: OperationPath<'revokeOperatorRole'>['role'],
  body: OperationBody<'revokeOperatorRole'>,
) {
  return postJson<OperationBody<'revokeOperatorRole'>, OperationOk<'revokeOperatorRole'>>(
    credentials,
    `/admin/identity-governance-center/operator-grants/${principalId}/${role}/revoke`,
    body,
  );
}

export async function getTenantDetail(credentials: AdminCredentials, tenantId: string) {
  const response = await verifiedFetch(`/admin/tenants/${tenantId}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<TenantDetail>(response);
}

export function suspendTenant(
  credentials: AdminCredentials,
  tenantId: OperationPath<'suspendTenant'>['tenantId'],
  body: OperationBody<'suspendTenant'>,
) {
  return postJson<OperationBody<'suspendTenant'>, OperationOk<'suspendTenant'>>(
    credentials,
    `/admin/tenants/${tenantId}/suspend`,
    body,
  );
}

export function reactivateTenant(
  credentials: AdminCredentials,
  tenantId: OperationPath<'reactivateTenant'>['tenantId'],
  body: OperationBody<'reactivateTenant'>,
) {
  return postJson<OperationBody<'reactivateTenant'>, OperationOk<'reactivateTenant'>>(
    credentials,
    `/admin/tenants/${tenantId}/reactivate`,
    body,
  );
}

export async function getWorkspaceDetail(credentials: AdminCredentials, workspaceId: string) {
  const response = await verifiedFetch(`/admin/workspaces/${workspaceId}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<WorkspaceDetail>(response);
}

export function suspendWorkspace(
  credentials: AdminCredentials,
  workspaceId: OperationPath<'suspendWorkspace'>['workspaceId'],
  body: OperationBody<'suspendWorkspace'>,
) {
  return postJson<OperationBody<'suspendWorkspace'>, OperationOk<'suspendWorkspace'>>(
    credentials,
    `/admin/workspaces/${workspaceId}/suspend`,
    body,
  );
}

export function reactivateWorkspace(
  credentials: AdminCredentials,
  workspaceId: OperationPath<'reactivateWorkspace'>['workspaceId'],
  body: OperationBody<'reactivateWorkspace'>,
) {
  return postJson<OperationBody<'reactivateWorkspace'>, OperationOk<'reactivateWorkspace'>>(
    credentials,
    `/admin/workspaces/${workspaceId}/reactivate`,
    body,
  );
}

export async function getUserDetail(credentials: AdminCredentials, principalId: string) {
  const response = await verifiedFetch(`/admin/users/${principalId}`, {
    headers: authHeaders(credentials),
  });
  return parseJson<UserDetail>(response);
}

export function suspendUser(
  credentials: AdminCredentials,
  principalId: OperationPath<'suspendUser'>['principalId'],
  body: OperationBody<'suspendUser'>,
) {
  return postJson<OperationBody<'suspendUser'>, OperationOk<'suspendUser'>>(
    credentials,
    `/admin/users/${principalId}/suspend`,
    body,
  );
}

export function reactivateUser(
  credentials: AdminCredentials,
  principalId: OperationPath<'reactivateUser'>['principalId'],
  body: OperationBody<'reactivateUser'>,
) {
  return postJson<OperationBody<'reactivateUser'>, OperationOk<'reactivateUser'>>(
    credentials,
    `/admin/users/${principalId}/reactivate`,
    body,
  );
}
