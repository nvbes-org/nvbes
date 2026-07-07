import * as ids from './action-confirmation.fixtures';

export function commandSnapshot() {
  return {
    tenant_count: 1,
    workspace_count: 1,
    user_count: 1,
    audit_events_24h: 0,
    pending_approval_count: 3,
    critical_pending_approval_count: 1,
    overdue_approval_count: 1,
    open_incident_count: 1,
    audit_hash_anomaly_count: 1,
    sla_breach_count: 2,
    billing_provider_failures: 1,
    overdue_invoice_count: 1,
    failed_payment_count: 0,
    latest_audit_at: null,
    today_work: [
      {
        id: 'pending-approvals',
        label: 'Pending approvals',
        count: 3,
        severity: 'critical',
        href: '#pending-approvals',
        owner: 'Ops lead',
      },
      {
        id: 'open-incidents',
        label: 'Open incidents',
        count: 1,
        severity: 'critical',
        href: '#operations-center',
        owner: 'Operations',
      },
      {
        id: 'audit-anomalies',
        label: 'Audit anomalies',
        count: 1,
        severity: 'critical',
        href: '#audit-evidence-center',
        owner: 'Security',
      },
      {
        id: 'sla-breaches',
        label: 'SLA pressure',
        count: 2,
        severity: 'high',
        href: '#revenue-center',
        owner: 'Revenue ops',
      },
    ],
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
    active_provider_count: 1,
    active_provider_account_count: 2,
    active_routing_rule_count: 1,
    fallback_routing_rule_count: 1,
    planned_migration_count: 1,
    pending_kyc_profile_count: 1,
    region_policy_count: 1,
    active_einvoicing_profile_count: 1,
    providers: [
      {
        provider: 'stripe',
        status: 'active',
        account_count: 2,
        updated_at: '2026-06-24T09:30:00Z',
      },
    ],
    routing_rules: [
      {
        id: ids.routingRuleId,
        priority: 10,
        provider: 'stripe',
        country: 'FR',
        currency: 'EUR',
        payment_method: 'card',
        customer_type: 'business',
        fallback_enabled: true,
        status: 'active',
      },
    ],
    provider_migrations: [
      {
        id: '56565656-5656-4565-8565-565656565656',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        from_provider: 'legacy_psp',
        to_provider: 'stripe',
        status: 'planned',
        started_at: null,
        updated_at: '2026-06-24T11:00:00Z',
      },
    ],
    kyc_profiles: [
      {
        id: '57575757-5757-4575-8575-575757575757',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        company_name: 'Acme Europe SAS',
        company_domain: 'acme.example',
        vat_id: 'FR12345678901',
        proof_reference: 'proof-kyc-2026-06',
        review_status: 'pending_review',
        updated_at: '2026-06-24T10:45:00Z',
      },
    ],
    region_policies: [
      {
        id: '58585858-5858-4585-8585-585858585858',
        country: 'FR',
        currency: 'EUR',
        allowed_payment_methods: ['card', 'sepa_debit'],
        invoice_retention_years: 10,
        tax_evidence_required: true,
        einvoicing_profile_code: 'fr-chorus-pro',
      },
    ],
    einvoicing_profiles: [
      {
        id: '59595959-5959-4595-8595-595959595959',
        code: 'fr-chorus-pro',
        country: 'FR',
        format: 'factur-x',
        status: 'active',
        updated_at: '2026-06-24T08:00:00Z',
      },
    ],
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
    queued_message_count: 2,
    sent_message_count_24h: 40,
    delivered_message_count_24h: 37,
    failed_message_count_24h: 1,
    suppressed_email_count: 1,
    webhook_event_count_24h: 3,
    unprocessed_event_count: 1,
    status_distribution: [
      {
        status: 'failed',
        message_count: 1,
      },
      {
        status: 'delivered',
        message_count: 37,
      },
    ],
    business_type_distribution: [
      {
        business_type: 'invoice_notice',
        message_count: 12,
        failure_count: 1,
      },
    ],
    recent_failures: [
      {
        id: ids.emailMessageId,
        business_type: 'invoice_notice',
        recipient_email: 'billing@example.test',
        provider_email_id: 'msg_failed_123',
        status: 'failed',
        updated_at: '2026-06-24T10:05:00Z',
      },
    ],
    recent_suppressions: [
      {
        email: 'blocked@example.test',
        reason: 'hard_bounce',
        suppressed_at: '2026-06-24T09:45:00Z',
      },
    ],
    recent_unprocessed_events: [
      {
        id: '34343434-3434-4343-8343-343434343434',
        provider_event_id: 'evt_email_unprocessed',
        provider_email_id: 'msg_failed_123',
        email: 'billing@example.test',
        event_type: 'bounce',
        occurred_at: '2026-06-24T10:04:00Z',
        created_at: '2026-06-24T10:06:00Z',
      },
    ],
  };
}

export function complianceSnapshot() {
  return {
    active_consent_count: 12,
    revoked_consent_count_30d: 1,
    suppressed_email_count: 2,
    email_bounce_count_24h: 1,
    email_delivery_failure_count_24h: 1,
    unverified_user_count: 1,
    recent_revoked_consents: [
      {
        id: ids.consentId,
        principal_id: ids.principalId,
        email: 'customer@example.test',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        consent_type: 'marketing',
        document_version: 'privacy-2026-06',
        revoked_at: '2026-06-24T10:15:00Z',
      },
    ],
    recent_suppressed_emails: [
      {
        email: 'bounce@example.test',
        reason: 'hard_bounce',
        principal_id: ids.principalId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        suppressed_at: '2026-06-24T10:20:00Z',
      },
      {
        email: 'orphan@example.test',
        reason: 'manual_suppression',
        principal_id: null,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        suppressed_at: '2026-06-24T09:20:00Z',
      },
    ],
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
    active_client_count: 2,
    pending_marketplace_app_count: 1,
    failed_webhook_delivery_count_24h: 1,
    active_webhook_endpoint_count: 1,
    expiring_secret_count: 1,
    restricted_scope_count: 1,
    failing_health_check_count: 1,
    pending_marketplace_apps: [
      {
        id: '29292929-2929-4292-8292-292929292929',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        client_id: ids.developerClientId,
        client_name: 'Acme Marketplace Sync',
        status: 'pending_review',
        created_at: '2026-06-24T08:00:00Z',
        updated_at: '2026-06-24T10:00:00Z',
      },
    ],
    webhook_failures: [
      {
        id: '30303030-3030-4303-8303-303030303030',
        endpoint_id: '31313131-3131-4313-8313-313131313131',
        endpoint_name: 'billing-events-endpoint',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        event_type: 'invoice.paid',
        status: 'failed',
        attempt_count: 3,
        response_status: 500,
        error_message: 'upstream returned 500',
        created_at: '2026-06-24T10:15:00Z',
      },
    ],
    expiring_secrets: [
      {
        id: '32323232-3232-4323-8323-323232323232',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        client_id: ids.developerClientId,
        client_name: 'Acme Marketplace Sync',
        status: 'active',
        secret_last4: '9f2a',
        expires_at: '2026-06-30T23:59:59Z',
      },
    ],
    risky_scopes: [
      {
        scope_key: 'billing:write',
        display_name: 'Billing write access',
        risk: 'restricted',
        lifecycle: 'active',
        owner_team: 'platform',
        updated_at: '2026-06-24T09:00:00Z',
      },
    ],
    health_issues: [
      {
        id: '33333333-3333-4333-8333-333333333334',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        target_type: 'oauth_client',
        target_id: ids.developerClientId,
        check_kind: 'webhook_signature_validation',
        status: 'failing',
        summary: 'Webhook signature validation failed on last delivery.',
        checked_at: '2026-06-24T10:20:00Z',
      },
    ],
  };
}

export function entitlementsSnapshot() {
  return {
    active_plan_count: 1,
    active_feature_count: 4,
    quota_definition_count: 2,
    active_entitlement_count: 3,
    over_quota_balance_count: 1,
    unpublished_change_count: 1,
    active_trial_grant_count: 1,
    active_plans: [
      {
        plan_id: '23232323-2323-4232-8232-232323232323',
        product_name: 'Drive',
        plan_code: 'drive-scale',
        plan_name: 'Drive Scale',
        active_version_count: 2,
        feature_count: 4,
      },
    ],
    over_quota_balances: [
      {
        id: '24242424-2424-4242-8242-242424242424',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        workspace_id: ids.workspaceId,
        workspace_name: 'Acme HQ',
        quota_code: 'storage_gb',
        included_quantity: 100,
        used_quantity: 137,
        period_end: '2026-06-30T23:59:59Z',
      },
    ],
    expiring_entitlements: [
      {
        id: '25252525-2525-4252-8252-252525252525',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        workspace_id: ids.workspaceId,
        workspace_name: 'Acme HQ',
        status: 'trialing',
        effective_to: '2026-06-30T23:59:59Z',
      },
    ],
    unpublished_changes: [
      {
        id: '26262626-2626-4262-8262-262626262626',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        event_id: 'entitlement.catalog.plan.updated',
        created_at: '2026-06-24T10:00:00Z',
      },
    ],
  };
}

export function operationsSnapshot() {
  return {
    provider_event_failure_count: 1,
    provider_event_backlog_count: 0,
    export_pending_count: 1,
    export_failed_count: 1,
    reconciliation_pending_count: 0,
    reconciliation_failed_count: 1,
    unresolved_reconciliation_difference_count: 1,
    open_incident_count: 1,
    scheduled_maintenance_window_count: 0,
    failed_job_run_count: 1,
    queued_email_count: 0,
    dropped_email_count_24h: 0,
    audit_events_24h: 0,
    recent_provider_failures: [
      {
        id: ids.providerEventId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Corp',
        provider: 'stripe',
        provider_event_id: 'evt_failed_mock',
        event_type: 'invoice.payment_failed',
        status: 'failed',
        received_at: '2026-06-24T10:00:00Z',
      },
    ],
    recent_export_runs: [
      {
        id: '13131313-1313-4131-8131-131313131313',
        export_type: 'invoices',
        status: 'failed',
        period_start: '2026-06-01',
        period_end: '2026-06-24',
        created_at: '2026-06-24T09:00:00Z',
        updated_at: '2026-06-24T09:05:00Z',
      },
    ],
    recent_reconciliation_differences: [
      {
        id: '14141414-1414-4141-8141-141414141414',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Corp',
        difference_type: 'ledger_provider_amount_mismatch',
        severity: 'critical',
        created_at: '2026-06-24T08:00:00Z',
      },
    ],
  };
}

export function regionSnapshot() {
  return {
    eu_workspace_count: 1,
    non_eu_workspace_count: 1,
    gdpr_workspace_count: 1,
    non_gdpr_workspace_count: 1,
    multi_region_tenant_count: 1,
    region_distribution: [
      {
        data_region: 'eu',
        workspace_count: 1,
        tenant_count: 1,
      },
      {
        data_region: 'us',
        workspace_count: 1,
        tenant_count: 1,
      },
    ],
    jurisdiction_distribution: [
      {
        jurisdiction: 'gdpr',
        workspace_count: 1,
        tenant_count: 1,
      },
      {
        jurisdiction: 'ccpa',
        workspace_count: 1,
        tenant_count: 1,
      },
    ],
    non_eu_workspaces: [
      {
        workspace_id: ids.workspaceId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        workspace_name: 'Acme US Analytics',
        data_region: 'us',
        jurisdiction: 'ccpa',
        created_at: '2026-06-24T09:30:00Z',
      },
    ],
    multi_region_tenants: [
      {
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        workspace_count: 2,
        region_count: 2,
        regions: ['eu', 'us'],
      },
    ],
  };
}

export function revenueSnapshot() {
  return {
    captured_payments_30d: [{ amount_minor: 420000, currency: 'EUR', object_count: 3 }],
    open_invoices: [{ amount_minor: 180000, currency: 'EUR', object_count: 2 }],
    overdue_invoices: [{ amount_minor: 99000, currency: 'EUR', object_count: 1 }],
    refunds_30d: [],
    disputes_30d: [{ amount_minor: 45000, currency: 'EUR', object_count: 1 }],
    active_subscription_count: 0,
    trialing_subscription_count: 0,
    open_dunning_case_count: 1,
    unresolved_reconciliation_difference_count: 0,
    recent_dunning_cases: [
      {
        id: '15151515-1515-4151-8151-151515151515',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Corp',
        status: 'open',
        policy_state: 'escalated',
        opened_at: '2026-06-24T07:00:00Z',
      },
    ],
    recent_disputes: [
      {
        id: '16161616-1616-4161-8161-161616161616',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Corp',
        status: 'needs_response',
        currency: 'EUR',
        amount_minor: 45000,
        created_at: '2026-06-24T06:00:00Z',
      },
    ],
    recent_overdue_invoices: [
      {
        id: ids.invoiceId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Corp',
        invoice_number: 'INV-2026-0001',
        status: 'issued',
        currency: 'EUR',
        total_minor: 99000,
        due_at: '2026-06-20T00:00:00Z',
      },
    ],
    recent_captured_payments: [
      {
        id: '17171717-1717-4171-8171-171717171717',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Corp',
        status: 'captured',
        currency: 'EUR',
        amount_minor: 420000,
        created_at: '2026-06-24T05:00:00Z',
      },
    ],
  };
}

export function riskDecisionSnapshot() {
  return {
    identity_risk_event_count_24h: 1,
    high_identity_risk_event_count_24h: 1,
    billing_risk_signal_count_24h: 1,
    high_billing_risk_score_count: 1,
    active_access_policy_count: 1,
    access_policy_count_24h: 1,
    recent_identity_risks: [
      {
        id: '20202020-2020-4202-8202-202020202020',
        principal_id: ids.principalId,
        email: 'risky.user@example.test',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        event_type: 'impossible_travel',
        risk_score: 0.91,
        decision: 'step_up_required',
        created_at: '2026-06-24T10:30:00Z',
      },
    ],
    billing_risk_scores: [
      {
        id: '21212121-2121-4212-8212-212121212121',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        score: 89,
        decision: 'manual_review',
        created_at: '2026-06-24T10:20:00Z',
      },
    ],
    billing_risk_signals: [
      {
        id: ids.riskSignalId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        signal_type: 'provider_dispute_spike',
        occurred_at: '2026-06-24T10:10:00Z',
      },
    ],
    active_access_policies: [
      {
        id: ids.riskPolicyId,
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        workspace_id: ids.workspaceId,
        workspace_name: 'Acme HQ',
        policy_state: 'blocked',
        reason: 'high identity and billing risk correlation',
        effective_from: '2026-06-24T10:00:00Z',
        effective_to: null,
      },
    ],
  };
}

export function usageSnapshot() {
  return {
    active_meter_count: 2,
    usage_event_count_24h: 42,
    usage_quantity_24h: 1536,
    correction_count_30d: 1,
    rollup_count_current_period: 1,
    distinct_tenant_count_24h: 1,
    meter_usage_24h: [
      {
        meter_code: 'storage_gb',
        unit: 'gb',
        event_count: 24,
        quantity: 1280,
      },
    ],
    tenant_usage_24h: [
      {
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        event_count: 42,
        quantity: 1536,
      },
    ],
    recent_rollups: [
      {
        id: '27272727-2727-4272-8272-272727272727',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        workspace_id: ids.workspaceId,
        workspace_name: 'Acme HQ',
        meter_code: 'storage_gb',
        quantity: 1280,
        unit: 'gb',
        period_start: '2026-06-01T00:00:00Z',
        period_end: '2026-06-30T23:59:59Z',
      },
    ],
    recent_corrections: [
      {
        id: '28282828-2828-4282-8282-282828282828',
        tenant_id: ids.tenantId,
        tenant_name: 'Acme Europe',
        meter_code: 'api_calls',
        quantity_delta: -120,
        reason: 'duplicate provider import',
        created_by_principal_id: ids.actorId,
        created_at: '2026-06-24T10:00:00Z',
      },
    ],
  };
}
