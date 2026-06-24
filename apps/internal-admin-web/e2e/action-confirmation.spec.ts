import { expect, type Page, test } from '@playwright/test';

const workspaceId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const tenantId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const actorId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
const principalId = '11111111-1111-4111-8111-111111111111';
const mfaFactorId = '22222222-2222-4222-8222-222222222222';
const recoveryRequestId = '33333333-3333-4333-8333-333333333333';

test.describe('internal-admin actionable flows @mocked', () => {
  test.beforeEach(async ({ page }) => {
    await installCredentials(page);
    await installMockApi(page);
  });

  test('Access Center requires strong confirmation and sends idempotent mutation', async ({
    page,
  }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const article = page.locator('article').filter({ hasText: 'access-admin@example.test' });
    await article.getByRole('button', { name: 'Suspend access' }).click();

    await article.getByPlaceholder('Motif audit').fill('ticket IAM-123 approved');
    await article.getByPlaceholder('SUSPEND ACCESS 11111111').fill('SUSPEND ACCESS');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await article.getByPlaceholder('SUSPEND ACCESS 11111111').fill('SUSPEND ACCESS 11111111');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await article.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      article.getByText('internal_admin.access.workspace_membership.suspended'),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'SUSPEND ACCESS 11111111',
        hasIdempotencyKey: true,
        path: `/admin/access-center/workspace-memberships/${workspaceId}/${principalId}/suspend`,
      }),
    );
  });

  test('Security Center requires object-bound MFA confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const article = page.locator('article').filter({ hasText: 'security@example.test' }).first();
    await article.getByRole('button', { name: 'Revoquer' }).click();

    await article.getByPlaceholder('Motif audit').fill('ticket SEC-789 approved');
    await article.getByPlaceholder('REVOKE MFA 22222222').fill('REVOKE MFA');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await article.getByPlaceholder('REVOKE MFA 22222222').fill('REVOKE MFA 22222222');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await article.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText(`internal_admin.security.mfa_factor.revoked: ${mfaFactorId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE MFA 22222222',
        hasIdempotencyKey: true,
        path: `/admin/security-center/mfa-factors/${mfaFactorId}/revoke`,
      }),
    );
  });

  test('Identity Governance requires object-bound break-glass confirmation', async ({ page }) => {
    const posts: MutationPost[] = [];
    await captureMutation(page, posts);

    await page.goto('/');
    const article = page.locator('article').filter({ hasText: 'break glass emergency' });
    await article.getByRole('button', { name: 'Revoquer' }).click();

    await article.getByPlaceholder('Motif audit').fill('ticket GOV-456 approved');
    await article.getByPlaceholder('REVOKE BREAK GLASS 11111111').fill('REVOKE BREAK GLASS');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeDisabled();

    await article
      .getByPlaceholder('REVOKE BREAK GLASS 11111111')
      .fill('REVOKE BREAK GLASS 11111111');
    await expect(article.getByRole('button', { name: 'Confirmer' })).toBeEnabled();
    await article.getByRole('button', { name: 'Confirmer' }).click();

    await expect(
      page.getByText(`internal_admin.identity_governance.break_glass.revoked: ${principalId}`),
    ).toBeVisible();
    expect(posts).toContainEqual(
      expect.objectContaining({
        confirmCode: 'REVOKE BREAK GLASS 11111111',
        hasIdempotencyKey: true,
        path: `/admin/identity-governance-center/break-glass/${tenantId}/${principalId}/revoke`,
      }),
    );
  });
});

type MutationPost = {
  confirmCode: string;
  hasIdempotencyKey: boolean;
  path: string;
};

async function installCredentials(page: Page) {
  await page.addInitScript(
    ({ actorIdValue, tenantWorkspaceId }) => {
      window.localStorage.setItem(
        'nvbes.internal-admin.credentials',
        JSON.stringify({
          workspaceId: tenantWorkspaceId,
          internalToken: 'test-internal-token',
          actorPrincipalId: actorIdValue,
          backofficeRole: 'platform_admin',
          secondApproverPrincipalId: '',
          secondApproverRole: 'platform_admin',
        }),
      );
    },
    { actorIdValue: actorId, tenantWorkspaceId: workspaceId },
  );
}

async function installMockApi(page: Page) {
  await page.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith('/admin') && !url.pathname.startsWith('/workspaces')) {
      await route.continue();
      return;
    }

    if (request.method() === 'GET') {
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(snapshotForPath(url.pathname)),
      });
      return;
    }

    await route.continue();
  });
}

async function captureMutation(page: Page, posts: MutationPost[]) {
  await page.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (request.method() !== 'POST' || !url.pathname.startsWith('/admin')) {
      await route.fallback();
      return;
    }
    const payload = request.postDataJSON() as { confirm_code?: string };
    posts.push({
      confirmCode: payload.confirm_code ?? '',
      hasIdempotencyKey: Boolean(request.headers()['idempotency-key']),
      path: url.pathname,
    });
    await route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify(resultForMutation(url.pathname)),
    });
  });
}

function snapshotForPath(path: string) {
  if (path === '/admin/command-center') return commandSnapshot();
  if (path === '/admin/access-center') return accessSnapshot();
  if (path === '/admin/audit-evidence-center') return auditEvidenceSnapshot();
  if (path === '/admin/billing-platform-center') return billingPlatformSnapshot();
  if (path === '/admin/communications-center') return communicationsSnapshot();
  if (path === '/admin/compliance-center') return complianceSnapshot();
  if (path === '/admin/customer-center') return customerSnapshot();
  if (path === '/admin/developer-center') return developerSnapshot();
  if (path === '/admin/entitlements-center') return entitlementsSnapshot();
  if (path === '/admin/operations-center') return operationsSnapshot();
  if (path === '/admin/region-center') return regionSnapshot();
  if (path === '/admin/revenue-center') return revenueSnapshot();
  if (path === '/admin/risk-decision-center') return riskDecisionSnapshot();
  if (path === '/admin/security-center') return securitySnapshot();
  if (path === '/admin/identity-governance-center') return governanceSnapshot();
  if (path === '/admin/audit-events' || path.endsWith('/admin/audit-events')) return [];
  if (path === `/workspaces/${workspaceId}/billing/admin/overview`) return billingOverview();
  if (path === `/workspaces/${workspaceId}/billing/admin/provider-events/failures`) return [];
  if (path === '/admin/billing/runbooks') return [];
  return {};
}

function commandSnapshot() {
  return {
    tenant_count: 1,
    workspace_count: 1,
    user_count: 1,
    audit_events_24h: 0,
    billing_provider_failures: 0,
    overdue_invoice_count: 0,
    failed_payment_count: 0,
    latest_audit_at: null,
  };
}

function auditEvidenceSnapshot() {
  return {
    audit_events_24h: 0,
    actorless_event_count_24h: 0,
    sensitive_action_count_24h: 0,
    missing_hash_count: 0,
    backfilled_hash_count: 0,
    active_signing_key_count: 0,
    deprecated_signing_key_count: 0,
    revoked_signing_key_count: 0,
    recent_audit_events: [],
    actorless_events: [],
    hash_anomalies: [],
    signing_keys: [],
  };
}

function billingPlatformSnapshot() {
  return {
    active_provider_count: 0,
    active_provider_account_count: 0,
    active_routing_rule_count: 0,
    fallback_routing_rule_count: 0,
    planned_migration_count: 0,
    pending_kyc_profile_count: 0,
    region_policy_count: 0,
    active_einvoicing_profile_count: 0,
    providers: [],
    routing_rules: [],
    provider_migrations: [],
    kyc_profiles: [],
    region_policies: [],
    einvoicing_profiles: [],
  };
}

function billingOverview() {
  return {
    open_invoice_count: 0,
    overdue_invoice_count: 0,
    open_invoice_total_minor: 0,
    failed_provider_event_count: 0,
    pending_refund_count: 0,
    active_subscription_count: 0,
    captured_payment_total_minor_30d: 0,
    last_billing_audit_at: null,
  };
}

function communicationsSnapshot() {
  return {
    queued_message_count: 0,
    sent_message_count_24h: 0,
    delivered_message_count_24h: 0,
    failed_message_count_24h: 0,
    suppressed_email_count: 0,
    webhook_event_count_24h: 0,
    unprocessed_event_count: 0,
    status_distribution: [],
    business_type_distribution: [],
    recent_failures: [],
    recent_suppressions: [],
    recent_unprocessed_events: [],
  };
}

function complianceSnapshot() {
  return {
    active_consent_count: 0,
    revoked_consent_count_30d: 0,
    suppressed_email_count: 0,
    email_bounce_count_24h: 0,
    email_delivery_failure_count_24h: 0,
    unverified_user_count: 0,
    recent_revoked_consents: [],
    recent_suppressed_emails: [],
  };
}

function customerSnapshot() {
  return {
    active_tenant_count: 1,
    suspended_tenant_count: 0,
    dormant_workspace_count: 0,
    pending_invitation_count: 0,
    expired_invitation_count: 0,
    usage_events_24h: 0,
    storage_bytes_used: 0,
    file_count: 0,
    high_storage_workspaces: [],
    dormant_workspaces: [],
    tenants_with_pending_invites: [],
  };
}

function developerSnapshot() {
  return {
    active_client_count: 0,
    pending_marketplace_app_count: 0,
    failed_webhook_delivery_count_24h: 0,
    active_webhook_endpoint_count: 0,
    expiring_secret_count: 0,
    restricted_scope_count: 0,
    failing_health_check_count: 0,
    pending_marketplace_apps: [],
    webhook_failures: [],
    expiring_secrets: [],
    risky_scopes: [],
    health_issues: [],
  };
}

function entitlementsSnapshot() {
  return {
    active_plan_count: 0,
    active_feature_count: 0,
    quota_definition_count: 0,
    active_entitlement_count: 0,
    over_quota_balance_count: 0,
    unpublished_change_count: 0,
    active_trial_grant_count: 0,
    active_plans: [],
    over_quota_balances: [],
    expiring_entitlements: [],
    unpublished_changes: [],
  };
}

function operationsSnapshot() {
  return {
    provider_event_failure_count: 0,
    provider_event_backlog_count: 0,
    export_pending_count: 0,
    export_failed_count: 0,
    reconciliation_pending_count: 0,
    reconciliation_failed_count: 0,
    unresolved_reconciliation_difference_count: 0,
    queued_email_count: 0,
    dropped_email_count_24h: 0,
    audit_events_24h: 0,
    recent_provider_failures: [],
    recent_export_runs: [],
    recent_reconciliation_differences: [],
  };
}

function regionSnapshot() {
  return {
    eu_workspace_count: 0,
    non_eu_workspace_count: 0,
    gdpr_workspace_count: 0,
    non_gdpr_workspace_count: 0,
    multi_region_tenant_count: 0,
    region_distribution: [],
    jurisdiction_distribution: [],
    non_eu_workspaces: [],
    multi_region_tenants: [],
  };
}

function revenueSnapshot() {
  return {
    captured_payments_30d: [],
    open_invoices: [],
    overdue_invoices: [],
    refunds_30d: [],
    disputes_30d: [],
    active_subscription_count: 0,
    trialing_subscription_count: 0,
    open_dunning_case_count: 0,
    unresolved_reconciliation_difference_count: 0,
    recent_dunning_cases: [],
    recent_disputes: [],
    recent_overdue_invoices: [],
    recent_captured_payments: [],
  };
}

function riskDecisionSnapshot() {
  return {
    identity_risk_event_count_24h: 0,
    high_identity_risk_event_count_24h: 0,
    billing_risk_signal_count_24h: 0,
    high_billing_risk_score_count: 0,
    active_access_policy_count: 0,
    access_policy_count_24h: 0,
    recent_identity_risks: [],
    billing_risk_scores: [],
    billing_risk_signals: [],
    active_access_policies: [],
  };
}

function accessSnapshot() {
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
        principal_id: principalId,
        email: 'access-admin@example.test',
        name: 'Access Admin',
        tenant_id: tenantId,
        tenant_name: 'Acme',
        workspace_id: workspaceId,
        workspace_name: 'Acme Workspace',
        role: 'admin',
        updated_at: '2026-06-24T10:00:00Z',
      },
    ],
    ownerless_workspaces: [],
    stale_service_accounts: [],
  };
}

function securitySnapshot() {
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
        id: mfaFactorId,
        principal_id: principalId,
        email: 'security@example.test',
        tenant_id: tenantId,
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

function governanceSnapshot() {
  return {
    active_idp_count: 0,
    unverified_domain_count: 0,
    active_scim_connector_count: 0,
    overdue_access_review_count: 0,
    pending_review_item_count: 0,
    active_break_glass_count: 1,
    pending_recovery_count: 1,
    unverified_domains: [],
    sso_providers: [],
    scim_connectors: [],
    overdue_access_reviews: [],
    break_glass_accounts: [
      {
        tenant_id: tenantId,
        tenant_name: 'Acme',
        principal_id: principalId,
        procedure_reference: 'BG-123',
        reason: 'break glass emergency',
        last_used_at: null,
        created_at: '2026-06-24T10:00:00Z',
      },
    ],
    pending_recovery_requests: [
      {
        id: recoveryRequestId,
        tenant_id: tenantId,
        tenant_name: 'Acme',
        principal_id: principalId,
        email: 'recovery@example.test',
        status: 'pending',
        available_at: '2026-06-24T10:00:00Z',
        created_at: '2026-06-24T09:00:00Z',
      },
    ],
  };
}

function resultForMutation(path: string) {
  if (path.includes('/access-center/')) {
    return {
      workspace_id: workspaceId,
      tenant_id: tenantId,
      principal_id: principalId,
      previous_status: 'active',
      next_status: 'suspended',
      audit_action: 'internal_admin.access.workspace_membership.suspended',
    };
  }
  if (path.includes('/security-center/')) {
    return {
      object_id: mfaFactorId,
      tenant_id: tenantId,
      principal_id: principalId,
      audit_action: 'internal_admin.security.mfa_factor.revoked',
    };
  }
  return {
    object_id: principalId,
    tenant_id: tenantId,
    principal_id: principalId,
    audit_action: 'internal_admin.identity_governance.break_glass.revoked',
  };
}
