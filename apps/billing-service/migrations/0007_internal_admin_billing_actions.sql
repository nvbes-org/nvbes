CREATE TABLE IF NOT EXISTS internal_admin_risk_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  action_kind TEXT NOT NULL,
  policy_snapshot_id UUID REFERENCES billing_access_policy_snapshots(id) ON DELETE SET NULL,
  risk_signal_id UUID REFERENCES billing_risk_signals(id) ON DELETE SET NULL,
  previous_state TEXT,
  next_state TEXT NOT NULL,
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_risk_actions_tenant_created
  ON internal_admin_risk_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_risk_actions_workspace_created
  ON internal_admin_risk_actions (workspace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS internal_admin_operations_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  action_kind TEXT NOT NULL,
  provider_event_id UUID REFERENCES billing_provider_events(id) ON DELETE SET NULL,
  export_run_id UUID REFERENCES billing_export_runs(id) ON DELETE SET NULL,
  reconciliation_difference_id UUID REFERENCES billing_reconciliation_differences(id) ON DELETE SET NULL,
  previous_state TEXT,
  next_state TEXT NOT NULL,
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_operations_actions_tenant_created
  ON internal_admin_operations_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_operations_actions_workspace_created
  ON internal_admin_operations_actions (workspace_id, created_at DESC);

ALTER TABLE billing_invoices
  ADD COLUMN IF NOT EXISTS internal_hold_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS internal_hold_reason TEXT,
  ADD COLUMN IF NOT EXISTS internal_hold_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS internal_admin_revenue_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  action_kind TEXT NOT NULL,
  dunning_case_id UUID REFERENCES billing_dunning_cases(id) ON DELETE SET NULL,
  invoice_id UUID REFERENCES billing_invoices(id) ON DELETE SET NULL,
  dispute_id UUID REFERENCES billing_disputes(id) ON DELETE SET NULL,
  previous_state TEXT,
  next_state TEXT NOT NULL,
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_revenue_actions_tenant_created
  ON internal_admin_revenue_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_revenue_actions_workspace_created
  ON internal_admin_revenue_actions (workspace_id, created_at DESC);

ALTER TABLE billing_kyc_profiles
  ADD COLUMN IF NOT EXISTS review_status TEXT NOT NULL DEFAULT 'pending',
  ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS reviewed_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS review_reason TEXT;

CREATE TABLE IF NOT EXISTS internal_admin_billing_platform_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  action_kind TEXT NOT NULL,
  routing_rule_id UUID REFERENCES billing_provider_routing_rules(id) ON DELETE SET NULL,
  kyc_profile_id UUID REFERENCES billing_kyc_profiles(id) ON DELETE SET NULL,
  einvoicing_profile_id UUID REFERENCES billing_einvoicing_profiles(id) ON DELETE SET NULL,
  previous_state TEXT,
  next_state TEXT NOT NULL,
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_billing_platform_actions_tenant_created
  ON internal_admin_billing_platform_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_billing_platform_actions_workspace_created
  ON internal_admin_billing_platform_actions (workspace_id, created_at DESC);
