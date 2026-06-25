import * as ids from './action-confirmation.fixtures';

export function commandSnapshot() {
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

export function auditEvidenceSnapshot() {
  return {
    audit_events_24h: 0,
    actorless_event_count_24h: 0,
    sensitive_action_count_24h: 0,
    missing_hash_count: 0,
    backfilled_hash_count: 0,
    linked_hash_count: 1,
    chain_head_count: 0,
    hash_anomaly_count: 1,
    active_signing_key_count: 0,
    deprecated_signing_key_count: 0,
    revoked_signing_key_count: 0,
    recent_audit_events: [],
    actorless_events: [],
    hash_anomalies: [],
    signing_keys: [],
    alerts: [
      {
        id: 'missing_hash',
        severity: 'critical',
        title: 'Audit events without immutable hash',
        count: 1,
        target_anchor: '#audit-evidence-center',
      },
    ],
    runtime_alerts: [
      {
        id: 'backoffice_mutation_failures',
        severity: 'critical',
        title: 'Back-office mutations are failing',
        metric_name: 'internal_admin_action_requests_total',
        condition:
          'increase(metric{policy=~"mutation|critical_mutation",outcome="failed"}[5m]) > 0',
      },
    ],
  };
}

export function auditEventsSnapshot() {
  return [
    {
      id: 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee',
      tenant_id: ids.tenantId,
      workspace_id: ids.workspaceId,
      action: 'internal_admin.workspace.suspend',
      actor_principal_id: ids.actorId,
      actor_email: 'operator@example.test',
      target_type: 'workspace',
      target_id: ids.workspaceId,
      target_link: {
        kind: 'workspace',
        id: ids.workspaceId,
        href: '#workspace-detail',
        label: `workspace:${ids.workspaceId}`,
      },
      changes: [{ field: 'status', before: 'active', after: 'suspended' }],
      metadata: {
        reason: 'ticket AUD-123 approved',
        changes: [{ field: 'status', before: 'active', after: 'suspended' }],
      },
      event_hash: 'abcdef1234567890',
      previous_event_hash: '0123456789abcdef',
      hash_chain_status: 'linked',
      created_at: '2026-06-24T10:00:00Z',
    },
    {
      id: 'ffffffff-ffff-4fff-8fff-ffffffffffff',
      tenant_id: ids.tenantId,
      workspace_id: ids.workspaceId,
      action: 'internal_admin.audit.backfilled',
      actor_principal_id: null,
      actor_email: null,
      target_type: 'tenant',
      target_id: ids.tenantId,
      target_link: {
        kind: 'tenant',
        id: ids.tenantId,
        href: '#tenant-detail',
        label: `tenant:${ids.tenantId}`,
      },
      changes: [],
      metadata: { reason: 'hash backfill verification' },
      event_hash: 'backfill',
      previous_event_hash: null,
      hash_chain_status: 'hash_anomaly',
      created_at: '2026-06-24T09:00:00Z',
    },
  ];
}

export function auditEvidenceExportSnapshot() {
  const events = auditEventsSnapshot();
  return {
    export_id: '99999999-9999-4999-8999-999999999999',
    generated_at: '2026-06-24T10:05:00Z',
    tenant_id: ids.tenantId,
    workspace_id: ids.workspaceId,
    event_count: events.length,
    filters: {
      action: null,
      target_type: null,
      q: null,
      limit: 500,
    },
    hash_chain: {
      head_event_hash: events[0]?.event_hash ?? null,
      tail_event_hash: events.at(-1)?.event_hash ?? null,
      anomaly_count: 0,
      linked_count: events.length,
    },
    events,
  };
}

export function billingPlatformSnapshot() {
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

export function billingRunbooksSnapshot() {
  return [
    {
      id: 'psp-outage',
      title: 'PSP outage',
      severity: 'critical',
      steps: ['Basculer le routage provider.', 'Verifier les retries.'],
    },
  ];
}

export function billingOverview() {
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

export function communicationsSnapshot() {
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

export function complianceSnapshot() {
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

export function customerSnapshot() {
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

export function developerSnapshot() {
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

export function entitlementsSnapshot() {
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

export function operationsSnapshot() {
  return {
    provider_event_failure_count: 0,
    provider_event_backlog_count: 0,
    export_pending_count: 0,
    export_failed_count: 0,
    reconciliation_pending_count: 0,
    reconciliation_failed_count: 0,
    unresolved_reconciliation_difference_count: 0,
    open_incident_count: 1,
    scheduled_maintenance_window_count: 0,
    failed_job_run_count: 1,
    queued_email_count: 0,
    dropped_email_count_24h: 0,
    audit_events_24h: 0,
    recent_provider_failures: [],
    recent_export_runs: [],
    recent_reconciliation_differences: [],
  };
}

export function regionSnapshot() {
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

export function revenueSnapshot() {
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

export function riskDecisionSnapshot() {
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
