import * as ids from './action-confirmation.fixtures';

export function accessSnapshot() {
  return {
    workspace_owner_count: 1,
    workspace_admin_count: 1,
    ownerless_workspace_count: 0,
    service_account_count: 0,
    stale_service_account_count: 0,
    oauth_client_count: 0,
    revoked_oauth_client_count: 0,
    restricted_client_policy_count: 0,
    privileged_users: [
      {
        principal_id: ids.principalId,
        email: 'access-admin@example.test',
        name: 'Access Admin',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme',
        workspace_id: ids.workspaceId,
        workspace_name: 'Acme Workspace',
        role: 'admin',
        updated_at: '2026-06-24T10:00:00Z',
      },
    ],
    ownerless_workspaces: [],
    stale_service_accounts: [],
  };
}

export function securitySnapshot() {
  return {
    risk_events_24h: 0,
    high_risk_events_24h: 0,
    active_users_without_mfa: 0,
    suspended_principal_count: 0,
    revoked_principal_count: 0,
    unverified_user_count: 0,
    active_oauth_consent_count: 0,
    recent_risk_events: [],
    users_without_mfa: [],
    active_mfa_factors: [
      {
        id: ids.mfaFactorId,
        principal_id: ids.principalId,
        email: 'security@example.test',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme',
        factor_type: 'totp',
        label: 'Primary',
        last_used_at: null,
        created_at: '2026-06-24T10:00:00Z',
      },
    ],
    active_oauth_consents: [],
  };
}

export function governanceSnapshot() {
  return {
    active_idp_count: 0,
    unverified_domain_count: 0,
    active_scim_connector_count: 0,
    overdue_access_review_count: 0,
    pending_review_item_count: 0,
    active_break_glass_count: 1,
    pending_recovery_count: 1,
    active_operator_grant_count: 2,
    revoked_operator_grant_count: 1,
    unverified_domains: [],
    sso_providers: [],
    scim_connectors: [],
    overdue_access_reviews: [],
    break_glass_accounts: [
      {
        tenant_id: ids.tenantId,
        tenant_name: 'Acme',
        principal_id: ids.principalId,
        procedure_reference: 'BG-123',
        reason: 'break glass emergency',
        last_used_at: null,
        created_at: '2026-06-24T10:00:00Z',
      },
    ],
    pending_recovery_requests: [
      {
        id: ids.recoveryRequestId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme',
        principal_id: ids.principalId,
        email: 'recovery@example.test',
        status: 'pending',
        available_at: '2026-06-24T10:00:00Z',
        created_at: '2026-06-24T09:00:00Z',
      },
    ],
    operator_role_distribution: [
      { role: 'platform_admin', active_count: 1 },
      { role: 'security_admin', active_count: 1 },
    ],
    operator_grants: [
      {
        principal_id: ids.actorId,
        email: 'operator@example.test',
        display_name: 'Platform Operator',
        role: 'platform_admin',
        status: 'active',
        granted_at: '2026-06-24T08:00:00Z',
        revoked_at: null,
        reason: 'bootstrap access',
      },
      {
        principal_id: ids.secondApproverId,
        email: 'approver@example.test',
        display_name: 'Security Approver',
        role: 'security_admin',
        status: 'active',
        granted_at: '2026-06-24T08:10:00Z',
        revoked_at: null,
        reason: 'dual-control reviewer',
      },
      {
        principal_id: ids.principalId,
        email: 'former-operator@example.test',
        display_name: 'Former Operator',
        role: 'viewer',
        status: 'revoked',
        granted_at: '2026-06-20T08:00:00Z',
        revoked_at: '2026-06-24T08:00:00Z',
        reason: 'offboarded',
      },
    ],
  };
}
