import * as ids from './action-confirmation.fixtures';

export function globalSearchSnapshot() {
  return [
    {
      kind: 'tenant',
      id: ids.tenantId,
      label: 'Acme',
      status: 'active',
      tenant_id: ids.tenantId,
      workspace_id: null,
    },
    {
      kind: 'workspace',
      id: ids.workspaceId,
      label: 'Acme Workspace',
      status: 'active',
      tenant_id: ids.tenantId,
      workspace_id: ids.workspaceId,
    },
    {
      kind: 'user',
      id: ids.principalId,
      label: 'customer-user@example.test',
      status: 'active',
      tenant_id: ids.tenantId,
      workspace_id: ids.workspaceId,
    },
  ];
}

export function tenantDetailSnapshot() {
  return {
    id: ids.tenantId,
    name: 'Acme',
    slug: 'acme',
    status: 'active',
    kind: 'customer',
    security_tier: 'standard',
    workspace_count: 1,
    user_count: 1,
    audit_events_24h: 0,
    open_invoice_count: 0,
    provider_failure_count: 0,
    created_at: '2026-06-24T09:00:00Z',
    updated_at: '2026-06-24T10:00:00Z',
  };
}

export function workspaceDetailSnapshot() {
  return {
    id: ids.workspaceId,
    tenant_id: ids.tenantId,
    tenant_name: 'Acme',
    name: 'Acme Workspace',
    status: 'active',
    workspace_type: 'business',
    plan_code: 'pro',
    trial_ends_at: null,
    member_count: 1,
    owner_count: 1,
    active_member_count: 1,
    service_account_count: 0,
    audit_events_24h: 0,
    open_invoice_count: 0,
    active_subscription_count: 1,
    latest_audit_at: null,
    created_at: '2026-06-24T09:00:00Z',
    updated_at: '2026-06-24T10:00:00Z',
  };
}

export function userDetailSnapshot() {
  return {
    principal_id: ids.principalId,
    tenant_id: ids.tenantId,
    tenant_name: 'Acme',
    email: 'customer-user@example.test',
    name: 'Customer User',
    principal_status: 'active',
    user_status: 'active',
    email_verified_at: '2026-06-24T09:30:00Z',
    workspace_count: 1,
    active_workspace_count: 1,
    active_mfa_factor_count: 1,
    active_oauth_consent_count: 0,
    risk_events_24h: 0,
    audit_events_24h: 0,
    latest_risk_at: null,
    latest_audit_at: null,
    primary_workspace_id: ids.workspaceId,
    primary_workspace_name: 'Acme Workspace',
    created_at: '2026-06-24T09:00:00Z',
    updated_at: '2026-06-24T10:00:00Z',
  };
}

export function resultForMutation(path: string) {
  if (path.startsWith('/admin/tenants/')) {
    return {
      tenant_id: ids.tenantId,
      previous_status: 'active',
      next_status: 'suspended',
      audit_action: 'internal_admin.tenant.suspend',
    };
  }
  if (path.startsWith('/admin/users/')) {
    return {
      principal_id: ids.principalId,
      tenant_id: ids.tenantId,
      previous_principal_status: 'active',
      previous_user_status: 'active',
      next_principal_status: 'suspended',
      next_user_status: 'suspended',
      audit_action: 'internal_admin.user.suspend',
    };
  }
  if (path.startsWith('/admin/workspaces/')) {
    return {
      workspace_id: ids.workspaceId,
      tenant_id: ids.tenantId,
      previous_status: 'active',
      next_status: 'suspended',
      audit_action: 'internal_admin.workspace.suspend',
    };
  }
  if (path.includes('/billing-platform/')) {
    return {
      object_id: ids.routingRuleId,
      status: 'disabled',
      audit_action: 'internal_admin.billing_platform.routing_rule.disabled',
    };
  }
  if (path.includes('/billing/admin/runbooks/')) {
    return {
      workspace_id: ids.workspaceId,
      tenant_id: ids.tenantId,
      runbook_id: 'psp-outage',
      audit_action: 'internal_admin.runbook.executed',
    };
  }
  if (path.includes('/billing/admin/')) {
    return {
      object_id: ids.invoiceId,
      status: 'created',
      audit_action: 'billing.credit_note.created',
    };
  }
  if (path.includes('/communications/')) {
    return {
      object_id: ids.emailMessageId,
      status: 'queued',
      audit_action: 'internal_admin.communications.email.replayed',
    };
  }
  if (path.includes('/compliance/consents/')) {
    return {
      object_id: ids.consentId,
      status: 'applied',
      audit_action: 'internal_admin.compliance.consent.revoked',
    };
  }
  if (path.includes('/compliance/principals/')) {
    return {
      object_id: ids.principalId,
      status: 'queued',
      audit_action: 'internal_admin.compliance.erasure.requested',
    };
  }
  if (path.includes('/compliance/suppressions/')) {
    return {
      object_id: 'abuse@example.test',
      status: 'reviewed',
      audit_action: 'internal_admin.compliance.suppression.reviewed',
    };
  }
  if (path.includes('/developer/')) {
    return {
      object_id: ids.developerClientId,
      status: 'revoked',
      audit_action: 'internal_admin.developer.client.revoked',
    };
  }
  if (path.includes('/entitlements/')) {
    return {
      object_id: 'advanced_search',
      status: 'queued',
      audit_action: 'internal_admin.entitlements.feature.granted',
    };
  }
  if (path.includes('/operations/')) {
    if (path.includes('/incidents/')) {
      return {
        object_id: ids.incidentId,
        status: 'mitigating',
        audit_action: 'internal_admin.operations.incident.state_updated',
      };
    }
    if (path.includes('/maintenance-windows')) {
      return {
        object_id: ids.workspaceId,
        status: 'scheduled',
        audit_action: 'internal_admin.operations.maintenance_window.scheduled',
      };
    }
    if (path.includes('/job-runs/')) {
      return {
        object_id: ids.jobRunId,
        status: 'queued',
        audit_action: 'internal_admin.operations.job_run.replayed',
      };
    }
    return {
      object_id: ids.providerEventId,
      status: 'queued',
      audit_action: 'internal_admin.operations.provider_event.replayed',
    };
  }
  if (path.includes('/region/') && path.endsWith('/residency-flag')) {
    return {
      object_id: ids.workspaceId,
      status: 'applied',
      audit_action: 'internal_admin.region.residency.flagged',
    };
  }
  if (path.includes('/region/')) {
    return {
      object_id: ids.workspaceId,
      status: 'recorded',
      audit_action: 'internal_admin.region.exception.recorded',
    };
  }
  if (path.includes('/revenue/')) {
    return {
      object_id: ids.invoiceId,
      status: 'held',
      audit_action: 'internal_admin.revenue.invoice.held',
    };
  }
  if (path.includes('/risk/signals/')) {
    return {
      object_id: ids.riskSignalId,
      status: 'resolved',
      audit_action: 'internal_admin.risk.signal.resolved',
    };
  }
  if (path.includes('/identity-governance-center/operator-grants/') && path.endsWith('/grant')) {
    return {
      object_id: ids.principalId,
      principal_id: ids.principalId,
      role: 'viewer',
      previous_status: null,
      next_status: 'active',
      audit_action: 'internal_admin.identity_governance.operator_grant.granted',
    };
  }
  if (path.includes('/identity-governance-center/operator-grants/')) {
    return {
      object_id: ids.actorId,
      principal_id: ids.actorId,
      role: 'platform_admin',
      previous_status: 'active',
      next_status: 'revoked',
      audit_action: 'internal_admin.identity_governance.operator_grant.revoked',
    };
  }
  if (path.includes('/risk/')) {
    return {
      object_id: ids.riskPolicyId,
      status: 'blocked',
      audit_action: 'internal_admin.risk.policy.blocked',
    };
  }
  if (path.includes('/usage/')) {
    return {
      object_id: 'api_call',
      status: 'applied',
      audit_action: 'internal_admin.usage.correction.created',
    };
  }
  if (path.includes('/access-center/')) {
    return {
      workspace_id: ids.workspaceId,
      tenant_id: ids.tenantId,
      principal_id: ids.principalId,
      previous_status: 'active',
      next_status: 'suspended',
      audit_action: 'internal_admin.access.workspace_membership.suspended',
    };
  }
  if (path.includes('/security-center/')) {
    return {
      object_id: ids.mfaFactorId,
      tenant_id: ids.tenantId,
      principal_id: ids.principalId,
      audit_action: 'internal_admin.security.mfa_factor.revoked',
    };
  }
  return {
    object_id: ids.principalId,
    tenant_id: ids.tenantId,
    principal_id: ids.principalId,
    audit_action: 'internal_admin.identity_governance.break_glass.revoked',
  };
}
